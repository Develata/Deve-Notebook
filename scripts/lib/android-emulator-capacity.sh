#!/usr/bin/env bash

parse_android_emulator_data_capacity() {
  tr -d '\r' | awk '
    NR == 1 { next }
    $NF == "/data" || index($NF, "/data/") == 1 {
      matches += 1
      if ($2 !~ /^[0-9]+$/ || $4 !~ /^[0-9]+$/ || $4 > $2) {
        invalid = 1
      }
      total_kib = $2
      available_kib = $4
    }
    END {
      if (matches != 1 || invalid) {
        exit 1
      }
      print total_kib, available_kib
    }
  '
}
