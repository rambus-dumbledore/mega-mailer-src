#!/bin/fish

docker build -t builder:latest -f docker/builder.docker .
docker build -t web_server:latest -f docker/web_server.docker .
docker build -t telegram_bot:latest -f docker/telegram_bot.docker .
docker build -t mail_checker:latest -f docker/mail_checker.docker .
