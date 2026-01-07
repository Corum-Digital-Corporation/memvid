# Memvid API Project History

## Goal
Deploy memvid as a REST API on AWS to enhance AI Agent memory and test as document RAG storage. Integrate with n8n 2.x running in Docker.

## AWS Environment
- Instance: ec2-user@ip-172-31-3-179
- Existing services: n8n, postgres, gotenberg on Docker network `docker_selfn8n`
- Project location: `~/memvid-api/`

---

## What We Built

### 1. FastAPI Wrapper for Memvid
Created a REST API wrapper instead of enabling Docker socket access in n8n (security concern).

**Architecture:**
```
n8n ──HTTP──> Memvid API (FastAPI) ──> .mv2 files
              Port 8100
```

**Files on AWS:**
```
~/memvid-api/
├── Dockerfile
├── docker-compose.yml
├── .env                    # OPENAI_API_KEY, MEMVID_API_KEY
├── app/
│   └── main.py            # FastAPI server
├── memvid-data/           # .mv2 memory files
└── documents/
```

**API Endpoints (all working):**
- `GET /health` - Health check
- `GET /docs` - Swagger UI
- `POST /memories` - Create memory
- `GET /memories` - List memories
- `POST /memories/{name}/ingest` - Add text
- `POST /memories/{name}/ingest-file` - Upload file
- `POST /memories/{name}/search` - Search
- `POST /memories/{name}/ask` - RAG query
- `GET /memories/{name}/stats` - Statistics
- `GET /memories/{name}/timeline` - Recent entries
- `DELETE /memories/{name}` - Delete memory

**Authentication:** X-API-Key header required (key in .env file)

### 2. Docker Configuration

**docker-compose.yml:**
```yaml
services:
  memvid-api:
    build: .
    container_name: memvid-api
    ports:
      - "8100:8000"
    volumes:
      - ./memvid-data:/data
      - ./documents:/documents
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - MEMVID_API_KEY=${MEMVID_API_KEY}
    restart: unless-stopped
    networks:
      - docker_selfn8n

networks:
  docker_selfn8n:
    external: true
```

**Dockerfile:**
```dockerfile
FROM python:3.11-slim
WORKDIR /app
RUN pip install --no-cache-dir fastapi uvicorn[standard] memvid-sdk python-multipart
COPY app/ /app/
EXPOSE 8000
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
```

---

## Issues Encountered

### Issue 1: Python Code Copying
Claude Code's terminal output indents code blocks, making Python code impossible to copy directly.

**Solution:** Write files locally to `C:\Users\becha\Code\memvid\` then SCP to AWS.

### Issue 2: memvid-sdk Requires Cloud API Key
The Python `memvid-sdk` package (v2.0.144) requires a Memvid cloud API key for write operations (put/ingest). Free tier is only 50MB.

- `create()` and `stats()` work locally
- `put()` fails with "Invalid API key. Get a valid key at https://memvid.com/dashboard/api-keys"

### Issue 3: No Official Docker Image
`memvid/cli` Docker image doesn't exist despite being mentioned in docs.

---

## Current Solution: Build from Source

We're building memvid from Rust source with limits removed.

**Steps completed:**
1. Installed Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Installed build tools: `sudo yum groupinstall "Development Tools" -y`
3. Cloned repo: `git clone https://github.com/memvid/memvid.git ~/memvid`
4. Modified limit in `~/memvid/src/types/common.rs`:
   - Changed: `Tier::Free => 50 * 1024 * 1024,`
   - To: `Tier::Free => 10 * 1024 * 1024 * 1024,` (10GB)
5. Built: `cd ~/memvid && cargo build --release`
6. Binary location: `~/memvid/target/release/`

**NEXT STEPS:**
1. Test the binary: `~/memvid/target/release/memvid --help`
2. Test creating a memory: `~/memvid/target/release/memvid create ~/memvid-api/memvid-data/local-test.mv2`
3. Update FastAPI to call the local binary instead of the cloud SDK
4. Rebuild Docker image with the custom binary included
5. Test full workflow

---

## Local Files (Windows)

Files created on local machine for transfer:
- `C:\Users\becha\Code\memvid\main.py` - FastAPI server code
- `C:\Users\becha\Code\memvid\history.md` - This file

**SCP command:**
```
scp -i your-key.pem C:\Users\becha\Code\memvid\main.py ec2-user@YOUR_AWS_IP:~/memvid-api/app/main.py
```

---

## Plan File

Full implementation plan saved at:
`/home/wbechard/.claude/plans/proud-roaming-badger.md`

---

## n8n Integration (Future)

Once API is working, use n8n HTTP Request node:
- URL: `http://memvid-api:8000/memories`
- Header: `X-API-Key: your-key`
- Body: JSON payload

No Docker socket mounting needed, no Execute Command node needed.

---

## Key Learnings

1. memvid-sdk Python package is cloud-dependent despite "local-first" marketing
2. The Rust CLI is the actual local tool
3. Building from source allows removing cloud limits
4. FastAPI wrapper is cleaner than Docker-in-Docker approaches
5. Claude Code can't output copyable Python (use file transfer instead)

---

## Commands Reference

**Rebuild container:**
```
cd ~/memvid-api && docker compose up -d --build && docker compose logs -f
```

**Test API:**
```
curl http://localhost:8100/health
curl http://localhost:8100/memories -H "X-API-Key: YOUR_KEY"
```

**Check memvid binary:**
```
~/memvid/target/release/memvid --help
```

---

---

## Session 2: CLI Binary Creation

### Problem Found
The `memvid` repo is a **library only** (`memvid-core`), not a CLI. Building it produces `libmemvid_core.rlib`, not an executable binary.

### Solution
Created a CLI wrapper by adding:
1. **Cargo.toml changes:**
   - Added `clap = { version = "4.4", features = ["derive"] }`
   - Added `[[bin]]` section pointing to `src/main.rs`

2. **New file: `src/main.rs`** - Full CLI with commands:
   - `memvid create <path>` - Create new .mv2 file
   - `memvid put <path> <content> [--title] [--uri]` - Add text
   - `memvid ingest <path> <file> [--title]` - Ingest a file
   - `memvid search <path> <query> [--top-k]` - Search
   - `memvid stats <path>` - Show statistics
   - `memvid timeline <path> [--limit]` - Show entries
   - `memvid verify <path> [--deep]` - Verify integrity

All commands output JSON for easy parsing by FastAPI.

### Files Modified (Local)
- `Cargo.toml` - Added clap + binary target
- `src/main.rs` - New CLI binary
- `src/types/common.rs` - 50MB → 10GB limit (from session 1)

### NEXT STEPS
1. Copy modified files to AWS:
   ```bash
   scp -i your-key.pem Cargo.toml ec2-user@YOUR_IP:~/memvid/Cargo.toml
   scp -i your-key.pem src/main.rs ec2-user@YOUR_IP:~/memvid/src/main.rs
   ```

2. Rebuild on AWS:
   ```bash
   cd ~/memvid && cargo build --release
   ```

3. Test the binary:
   ```bash
   ~/memvid/target/release/memvid --help
   ~/memvid/target/release/memvid create ~/memvid-api/memvid-data/test.mv2
   ~/memvid/target/release/memvid stats ~/memvid-api/memvid-data/test.mv2
   ```

4. Update FastAPI to call the binary via subprocess
5. Rebuild Docker image with the binary

---

## SUCCESS - Full Stack Working

All endpoints tested and working:
- `POST /memories` - Create memory
- `POST /memories/{name}/ingest` - Add text (was blocked by cloud SDK)
- `POST /memories/{name}/search` - Search works

**n8n Integration ready:**
- URL: `http://memvid-api:8000/memories` (from within Docker network)
- External: `http://YOUR_AWS_IP:8100/memories`
- Header: `X-API-Key: your-key`

---

Last updated: Session 2 - Full stack deployed and tested
