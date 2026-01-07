FROM python:3.11-slim

WORKDIR /app

# Install Python dependencies (no memvid-sdk needed!)
RUN pip install --no-cache-dir fastapi uvicorn[standard] python-multipart

# Copy the pre-built memvid binary
COPY memvid /app/memvid
RUN chmod +x /app/memvid

# Copy FastAPI app
COPY app/ /app/

# Create data directories
RUN mkdir -p /data /documents

ENV MEMVID_BIN=/app/memvid
ENV MEMVID_DATA_DIR=/data
ENV MEMVID_DOCUMENTS_DIR=/documents

EXPOSE 8000

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
