FROM python:3.11-slim

# Set up a non-root user for security
RUN groupadd --system app && useradd --system --gid app app
USER app

WORKDIR /home/app

COPY --chown=app:app ./dashboard/requirements.txt .
RUN pip install --no-cache-dir --user -r requirements.txt

COPY --chown=app:app . .

# Add the user's .local/bin to the PATH for Flask
ENV PATH="/home/app/.local/bin:${PATH}"
EXPOSE 5000

CMD ["flask", "run", "--host=0.0.0.0"]
