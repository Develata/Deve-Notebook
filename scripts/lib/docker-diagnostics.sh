#!/usr/bin/env bash
# Shared bounded diagnostics and full-stream parser plumbing for Docker smokes.

DOCKER_DIAGNOSTIC_MAX_BYTES=65536
DOCKER_DIAGNOSTIC_MAX_LINES=160
DOCKER_DIAGNOSTIC_MAX_LINE_BYTES=8192
DOCKER_DIAGNOSTIC_INPUT_MAX_BYTES=65536
DOCKER_DIAGNOSTIC_TOKEN_STATUS=3
DOCKER_DIAGNOSTIC_PRODUCER_FAILURE_STATUS=5
DOCKER_DIAGNOSTIC_MARKER="--- deve docker diagnostics: bounded tail bytes=${DOCKER_DIAGNOSTIC_MAX_BYTES} lines=${DOCKER_DIAGNOSTIC_MAX_LINES} ---"
DOCKER_LOG_PARSER_PATH="${DOCKER_LOG_PARSER_PATH:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/docker-log-parser.py}"

docker_log_parser_python() {
  local candidate="${DEVE_DOCKER_LOG_PARSER_PYTHON:-${PYTHON_BIN:-}}"
  if [[ -n "$candidate" ]]; then
    command -v "$candidate" >/dev/null 2>&1 || return 1
    printf '%s\n' "$candidate"
    return 0
  fi
  for candidate in python3 python; do
    if command -v "$candidate" >/dev/null 2>&1; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

# Add the producer's exit status after its output. The parser consumes this
# sentinel only after consuming the complete stream, so a successful-looking
# evidence line can never escape a failed producer through a command
# substitution. The NUL prefix is intentionally outside normal Docker text.
docker_stream_producer() {
  local producer_status
  "$@" 2>&1
  producer_status=$?
  printf '\n\0DEVE_DOCKER_PRODUCER_STATUS:%s\n' "$producer_status" || true
  return "$producer_status"
}

# Prefix the producer stream with a length-delimited token frame. Keeping the
# frame on stdin avoids external argv/env/disk disclosure and works with native
# Python launched from MSYS, where inherited non-standard file descriptors are
# not portable.
docker_stream_framed_producer() {
  local token_count="${1:-}"
  shift || true
  [[ "$token_count" =~ ^[0-8]$ ]] || return 125

  printf '\0DEVE_DOCKER_TOKEN_FRAME_V1:%s\n' "$token_count" || return 125
  local LC_ALL=C
  local i token token_bytes
  for ((i = 0; i < token_count; i++)); do
    (($# > 0)) || return 125
    token="$1"
    shift
    token_bytes="${#token}"
    ((token_bytes > 0 && token_bytes <= 8192)) || return 125
    printf '%s\n' "$token_bytes" || return 125
    printf '%s' "$token" || return 125
  done
  [[ "${1:-}" == "--" ]] || return 125
  shift
  docker_stream_producer "$@"
}

# Run one command through the allowlisted parser. Arguments before `--` are
# parser arguments; arguments after it are the producer command. The parser
# itself emits only a bounded diagnostic or a tiny allowlisted result.
docker_stream_parse_command() {
  local mode="${1:-}"
  shift || true
  local -a parser_args=()
  local -a producer=()
  local -a token_values=()
  local arg
  while (($# > 0)); do
    arg="$1"
    shift
    if [[ "$arg" == "--" ]]; then
      producer=("$@")
      break
    fi
    if [[ "$arg" == "--token" ]]; then
      (($# > 0)) || return 2
      ((${#token_values[@]} < 8)) || return 2
      token_values+=("$1")
      shift
      continue
    fi
    parser_args+=("$arg")
  done
  [[ -n "$mode" && ${#producer[@]} -gt 0 ]] || return 2

  local parser_python
  parser_python="$(docker_log_parser_python)" || return 127
  [[ -f "$DOCKER_LOG_PARSER_PATH" ]] || return 127

  local -a pipe_status=()
  if ((${#token_values[@]} > 0)); then
    if docker_stream_framed_producer "${#token_values[@]}" \
      "${token_values[@]}" -- "${producer[@]}" \
      | "$parser_python" "$DOCKER_LOG_PARSER_PATH" "$mode" \
        "${parser_args[@]}" --token-frame; then
      pipe_status=("${PIPESTATUS[@]}")
    else
      pipe_status=("${PIPESTATUS[@]}")
    fi
  else
    if docker_stream_producer "${producer[@]}" \
      | "$parser_python" "$DOCKER_LOG_PARSER_PATH" "$mode" "${parser_args[@]}"; then
      pipe_status=("${PIPESTATUS[@]}")
    else
      pipe_status=("${PIPESTATUS[@]}")
    fi
  fi

  local producer_status="${pipe_status[0]:-125}"
  local parser_status="${pipe_status[1]:-125}"
  # A producer failure is the primary failure. A parser/filter failure is
  # still fatal when the producer completed successfully.
  if (( producer_status != 0 )); then
    return "$DOCKER_DIAGNOSTIC_PRODUCER_FAILURE_STATUS"
  fi
  return "$parser_status"
}

# Direct stdin is used only by isolated parser regression tests. Production
# acceptance evidence always goes through docker_stream_parse_command so its
# producer status is bound to the result.
docker_stream_parse_stdin() {
  local mode="${1:-}"
  shift || true
  [[ -n "$mode" ]] || return 2
  local parser_python
  parser_python="$(docker_log_parser_python)" || return 127
  [[ -f "$DOCKER_LOG_PARSER_PATH" ]] || return 127
  "$parser_python" "$DOCKER_LOG_PARSER_PATH" "$mode" --direct "$@"
}

docker_bounded_command_output() {
  docker_stream_parse_command diagnostic -- "$@"
}

# Files are tailed before entering the parser. This keeps file diagnostics
# independent of file size and avoids materialising a complete raw log.
docker_bounded_file_output() {
  local file="${1:-}"
  [[ -f "$file" ]] || return 1
  docker_stream_parse_command diagnostic -- \
    tail -c "$DOCKER_DIAGNOSTIC_INPUT_MAX_BYTES" -- "$file"
}

docker_bounded_compose_logs() {
  local compose_command="${1:-docker_compose}"
  shift || true
  docker_stream_parse_command diagnostic -- \
    "$compose_command" logs --no-color --tail "$DOCKER_DIAGNOSTIC_MAX_LINES" "$@"
}
