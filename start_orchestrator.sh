#!/bin/bash

if [ -z "$1" ]; then
  config_path="./configs/default.json"
else
  config_path=$1
fi

ulimit -n 2048
. ./.venv/bin/activate
PYTHONUNBUFFERED=1 python3 orchestrator/main.py $config_path
