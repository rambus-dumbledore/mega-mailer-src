#!/bin/fish

docker build -t builder -f docker/builder.docker .
docker build -t web_server -f docker/web_server.docker .
docker build -t telegram_bot -f docker/telegram_bot.docker .
docker build -t mail_checker -f docker/mail_checker.docker .
