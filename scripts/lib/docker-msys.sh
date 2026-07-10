#!/usr/bin/env bash

# MSYS2 rewrites Unix-looking Docker arguments into Windows paths. Keep the
# override scoped to the docker run process so Docker build path conversion is
# unchanged and container paths such as /data/ledger remain literal.
docker_run_without_msys_arg_conversion() {
  if [[ "$#" -eq 0 ]]; then
    echo "docker-msys: missing docker run command" >&2
    return 2
  fi

  (
    export MSYS2_ARG_CONV_EXCL='*'
    "$@"
  )
}
