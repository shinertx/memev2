# Dockerfile for the analysis service
FROM python:3.10-slim

WORKDIR /app

# Install dependencies
COPY ./analysis/requirements.txt /app/
RUN pip install --no-cache-dir -r requirements.txt

# Copy the rest of the application code
COPY ./analysis /app/analysis
COPY ./shared /app/shared

# Command to run the service
CMD ["python", "-u", "analysis/alpha_decay_monitor.py"]
