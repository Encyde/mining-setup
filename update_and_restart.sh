#!/bin/bash

git fetch
git pull

. ./.venv/bin/activate
pip install -r requirements.txt

ps aux | grep " python3 orchestrator/main.py" | grep -v grep | awk '{print $2}' | xargs kill
ps aux | grep "uvicorn" | grep -v grep | awk '{print $2}' | xargs kill

sh ./start_local_config_server.sh $1
nohup ./start_orchestrator.sh > start_orchestrator.log 2>&1 &