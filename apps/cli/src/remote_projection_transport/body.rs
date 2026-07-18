//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract
//!
//! Bounded, provider-neutral delivery of one acquired payload into the
//! project-owned sink. Transport read failures and sink failures remain
//! distinguishable at the runtime boundary.

use super::{NormalizedRemotePath, RemoteSourceSink, SourceAcquisitionError};
use deve_core::remote_projection::RemoteProjectionProviderError;
use std::io::{self, Read};

pub(super) struct BodyCaptureBudget {
    pub(super) max_file_bytes: usize,
    pub(super) remaining_total_bytes: usize,
    pub(super) max_total_bytes: usize,
}

pub(super) fn capture_bounded_body<S: RemoteSourceSink>(
    provider_label: &str,
    path: &NormalizedRemotePath,
    body: &mut dyn Read,
    budget: BodyCaptureBudget,
    sink: &mut S,
) -> Result<usize, SourceAcquisitionError<S::Error>> {
    let total_limit_is_tighter = budget.remaining_total_bytes <= budget.max_file_bytes;
    let limit = budget.max_file_bytes.min(budget.remaining_total_bytes);
    let overflow_message = if total_limit_is_tighter {
        format!(
            "{provider_label} source acquisition exceeds total byte budget of {}",
            budget.max_total_bytes
        )
    } else {
        format!(
            "{provider_label} source payload exceeds {} bytes",
            budget.max_file_bytes
        )
    };
    let mut bounded = BoundedBodyReader::new(body, limit, provider_label, overflow_message);
    if let Err(error) = sink.capture(path, &mut bounded) {
        return match bounded.transport_error.take() {
            Some(error) => Err(SourceAcquisitionError::Transport(error)),
            None => Err(SourceAcquisitionError::Sink(error)),
        };
    }
    if let Some(error) = bounded.transport_error.take() {
        return Err(SourceAcquisitionError::Transport(error));
    }
    let mut probe = [0_u8; 1];
    match bounded.read(&mut probe) {
        Ok(0) => Ok(bounded.bytes_read),
        Ok(_) => Err(SourceAcquisitionError::Transport(
            RemoteProjectionProviderError::ProviderIo(format!(
                "{provider_label} source sink did not consume the complete payload for {}",
                path.as_str()
            )),
        )),
        Err(_) => Err(SourceAcquisitionError::Transport(
            bounded.transport_error.take().unwrap_or_else(|| {
                RemoteProjectionProviderError::ProviderIo(format!(
                    "{provider_label} source payload read failed for {}",
                    path.as_str()
                ))
            }),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote_projection_transport::RemoteSourceSink;

    struct SwallowingSink;

    impl RemoteSourceSink for SwallowingSink {
        type Error = std::convert::Infallible;

        fn capture(
            &mut self,
            _path: &NormalizedRemotePath,
            body: &mut dyn Read,
        ) -> Result<(), Self::Error> {
            let mut content = Vec::new();
            let _ignored = body.read_to_end(&mut content);
            Ok(())
        }
    }

    #[test]
    fn swallowed_reader_overflow_still_returns_transport_error() {
        let path = NormalizedRemotePath::new("a.md").expect("path");
        let mut body = io::Cursor::new(b"ab".to_vec());
        let error = capture_bounded_body(
            "test",
            &path,
            &mut body,
            BodyCaptureBudget {
                max_file_bytes: 1,
                remaining_total_bytes: 8,
                max_total_bytes: 8,
            },
            &mut SwallowingSink,
        )
        .expect_err("swallowed overflow");
        assert!(matches!(error, SourceAcquisitionError::Transport(_)));
        assert!(error.to_string().contains("source payload exceeds 1 bytes"));
    }
}

struct BoundedBodyReader<'a> {
    inner: &'a mut dyn Read,
    limit: usize,
    bytes_read: usize,
    provider_label: &'a str,
    overflow_message: String,
    transport_error: Option<RemoteProjectionProviderError>,
}

impl<'a> BoundedBodyReader<'a> {
    fn new(
        inner: &'a mut dyn Read,
        limit: usize,
        provider_label: &'a str,
        overflow_message: String,
    ) -> Self {
        Self {
            inner,
            limit,
            bytes_read: 0,
            provider_label,
            overflow_message,
            transport_error: None,
        }
    }

    fn record_io_error(&mut self, error: io::Error) -> io::Error {
        self.transport_error = Some(RemoteProjectionProviderError::ProviderIo(format!(
            "{} source payload read failed: {error}",
            self.provider_label
        )));
        error
    }
}

impl Read for BoundedBodyReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.bytes_read == self.limit {
            let mut extra = [0_u8; 1];
            return match self.inner.read(&mut extra) {
                Ok(0) => Ok(0),
                Ok(_) => {
                    let message = self.overflow_message.clone();
                    self.transport_error =
                        Some(RemoteProjectionProviderError::ProviderIo(message.clone()));
                    Err(io::Error::new(io::ErrorKind::InvalidData, message))
                }
                Err(error) => Err(self.record_io_error(error)),
            };
        }
        let remaining = self.limit - self.bytes_read;
        let requested = remaining.min(buffer.len());
        match self.inner.read(&mut buffer[..requested]) {
            Ok(read) => {
                self.bytes_read += read;
                Ok(read)
            }
            Err(error) => Err(self.record_io_error(error)),
        }
    }
}
