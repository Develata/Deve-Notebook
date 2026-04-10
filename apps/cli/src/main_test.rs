use super::{Args, Commands};
use clap::Parser;

#[test]
fn export_accepts_out_alias_for_output() {
    let args = Args::try_parse_from([
        "deve",
        "export",
        "--format",
        "markdown",
        "--doc",
        "123e4567-e89b-12d3-a456-426614174000",
        "--out",
        "/tmp/export.md",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Export {
            output,
            doc,
            format,
            ..
        }) => {
            assert_eq!(output.as_deref(), Some("/tmp/export.md"));
            assert_eq!(doc.as_deref(), Some("123e4567-e89b-12d3-a456-426614174000"));
            assert_eq!(format, "markdown");
        }
        other => panic!("unexpected command: {other:?}"),
    }
}
