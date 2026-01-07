"""
Memvid REST API - Wrapper for local memvid binary
"""
import json
import os
import subprocess
from pathlib import Path
from typing import Optional

from fastapi import FastAPI, HTTPException, Header, UploadFile, File
from pydantic import BaseModel

app = FastAPI(title="Memvid API", version="2.0.0")

# Configuration
MEMVID_BIN = os.getenv("MEMVID_BIN", "/app/memvid")
DATA_DIR = Path(os.getenv("MEMVID_DATA_DIR", "/data"))
DOCUMENTS_DIR = Path(os.getenv("MEMVID_DOCUMENTS_DIR", "/documents"))
API_KEY = os.getenv("MEMVID_API_KEY", "")


def verify_api_key(x_api_key: str = Header(None)):
    if API_KEY and x_api_key != API_KEY:
        raise HTTPException(status_code=401, detail="Invalid API key")


def run_memvid(*args) -> dict:
    """Run memvid binary and return JSON output."""
    cmd = [MEMVID_BIN] + list(args)
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
        if result.returncode != 0:
            raise HTTPException(status_code=500, detail=result.stderr or "Command failed")
        return json.loads(result.stdout)
    except subprocess.TimeoutExpired:
        raise HTTPException(status_code=504, detail="Command timed out")
    except json.JSONDecodeError:
        raise HTTPException(status_code=500, detail=f"Invalid JSON: {result.stdout}")


def get_memory_path(name: str) -> Path:
    """Get path to memory file, ensuring it's in DATA_DIR."""
    if "/" in name or "\\" in name:
        raise HTTPException(status_code=400, detail="Invalid memory name")
    return DATA_DIR / f"{name}.mv2"


# Models
class CreateMemoryRequest(BaseModel):
    name: str


class IngestTextRequest(BaseModel):
    content: str
    title: Optional[str] = None
    uri: Optional[str] = None


class SearchRequest(BaseModel):
    query: str
    top_k: int = 10


# Endpoints
@app.get("/health")
async def health():
    return {"status": "healthy", "binary": MEMVID_BIN}


@app.post("/memories")
async def create_memory(req: CreateMemoryRequest, x_api_key: str = Header(None)):
    verify_api_key(x_api_key)
    path = get_memory_path(req.name)
    if path.exists():
        raise HTTPException(status_code=409, detail="Memory already exists")
    result = run_memvid("create", str(path))
    return {"name": req.name, "status": result.get("status")}


@app.get("/memories")
async def list_memories(x_api_key: str = Header(None)):
    verify_api_key(x_api_key)
    memories = []
    for f in DATA_DIR.glob("*.mv2"):
        memories.append({"name": f.stem, "path": str(f)})
    return {"memories": memories}


@app.delete("/memories/{name}")
async def delete_memory(name: str, x_api_key: str = Header(None)):
    verify_api_key(x_api_key)
    path = get_memory_path(name)
    if not path.exists():
        raise HTTPException(status_code=404, detail="Memory not found")
    path.unlink()
    return {"status": "deleted", "name": name}


@app.post("/memories/{name}/ingest")
async def ingest_text(name: str, req: IngestTextRequest, x_api_key: str = Header(None)):
    verify_api_key(x_api_key)
    path = get_memory_path(name)
    if not path.exists():
        raise HTTPException(status_code=404, detail="Memory not found")

    args = ["put", str(path), req.content]
    if req.title:
        args.extend(["--title", req.title])
    if req.uri:
        args.extend(["--uri", req.uri])

    result = run_memvid(*args)
    return {"status": "ok", "sequence": result.get("sequence")}


@app.post("/memories/{name}/ingest-file")
async def ingest_file(
    name: str,
    file: UploadFile = File(...),
    title: Optional[str] = None,
    x_api_key: str = Header(None)
):
    verify_api_key(x_api_key)
    path = get_memory_path(name)
    if not path.exists():
        raise HTTPException(status_code=404, detail="Memory not found")

    # Save uploaded file temporarily
    temp_path = DOCUMENTS_DIR / file.filename
    content = await file.read()
    temp_path.write_bytes(content)

    try:
        args = ["ingest", str(path), str(temp_path)]
        if title:
            args.extend(["--title", title])
        result = run_memvid(*args)
        return {"status": "ok", "sequence": result.get("sequence"), "title": result.get("title")}
    finally:
        temp_path.unlink(missing_ok=True)


@app.post("/memories/{name}/search")
async def search_memory(name: str, req: SearchRequest, x_api_key: str = Header(None)):
    verify_api_key(x_api_key)
    path = get_memory_path(name)
    if not path.exists():
        raise HTTPException(status_code=404, detail="Memory not found")

    result = run_memvid("search", str(path), req.query, "--top-k", str(req.top_k))
    return result


@app.get("/memories/{name}/stats")
async def get_stats(name: str, x_api_key: str = Header(None)):
    verify_api_key(x_api_key)
    path = get_memory_path(name)
    if not path.exists():
        raise HTTPException(status_code=404, detail="Memory not found")

    result = run_memvid("stats", str(path))
    return {"name": name, **result}


@app.get("/memories/{name}/timeline")
async def get_timeline(name: str, limit: int = 20, x_api_key: str = Header(None)):
    verify_api_key(x_api_key)
    path = get_memory_path(name)
    if not path.exists():
        raise HTTPException(status_code=404, detail="Memory not found")

    result = run_memvid("timeline", str(path), "--limit", str(limit))
    return result
