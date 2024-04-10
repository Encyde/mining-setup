#!/bin/bash

. ./.venv/bin/activate

cd local_config_server
export HELIUS_URL=$1
nohup uvicorn --workers 1 main:app > ../local_config_server.log 2>&1 &
cd ../
