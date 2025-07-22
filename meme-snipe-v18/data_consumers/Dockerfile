# This Dockerfile can be used for all Python-based data consumers
FROM python:3.11-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .
# The CMD will be specified in the docker-compose.yml file
