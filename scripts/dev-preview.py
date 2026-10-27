#!/usr/bin/env python3
"""Manage only the local Vite preview owned by this checkout (Linux)."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import time
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parent.parent
APP = ROOT / "tauri-app"
RUNTIME = ROOT / ".runtime"
STATE = RUNTIME / "dev-preview.json"
VITE = APP / "node_modules/vite/bin/vite.js"


def identity(pid):
    proc = Path("/proc") / str(pid)
    try:
        stat = (proc / "stat").read_text().rsplit(") ", 1)[1].split()
        return {
            "start": stat[19],
            "cwd": str((proc / "cwd").resolve(strict=True)),
            "args": (proc / "cmdline").read_bytes().split(b"\0"),
            "env": (proc / "environ").read_bytes().split(b"\0"),
            "uid": proc.stat().st_uid,
        }
    except (FileNotFoundError, ProcessLookupError):
        return None


def owned(state):
    info = identity(state["pid"])
    if info is None:
        return False
    expected = f'MIHOMOAPP_PREVIEW_TOKEN={state["token"]}'.encode()
    if not (
        info["start"] == state["start"]
        and info["cwd"] == str(APP)
        and str(VITE).encode() in info["args"]
        and expected in info["env"]
        and info["uid"] == os.getuid()
        and os.getpgid(state["pid"]) == state["pid"]
    ):
        raise RuntimeError("PID 已被其他进程占用或归属无法确认，未执行终止操作")
    return True


def load_state():
    try:
        state = json.loads(STATE.read_text())
        if not all(key in state for key in ("pid", "start", "token", "port")):
            raise ValueError("missing fields")
        return state
    except FileNotFoundError:
        return None
    except (ValueError, TypeError) as error:
        raise RuntimeError("预览状态文件损坏，请先人工检查 .runtime/dev-preview.json") from error


def stop(state):
    if not owned(state):
        STATE.unlink(missing_ok=True)
        return
    os.killpg(state["pid"], signal.SIGTERM)
    for _ in range(50):
        if not owned(state):
            STATE.unlink(missing_ok=True)
            return
        time.sleep(0.1)
    # Revalidate the exact process identity before escalating.
    if owned(state):
        os.killpg(state["pid"], signal.SIGKILL)
    STATE.unlink(missing_ok=True)


def start(port):
    if not VITE.is_file():
        raise RuntimeError("缺少前端依赖，请先在 tauri-app 运行 npm ci")
    previous = load_state()
    if previous and owned(previous):
        if previous["port"] != port:
            raise RuntimeError(f'预览已运行于端口 {previous["port"]}，请先运行 ./stop.sh')
        print(f"预览已运行：http://127.0.0.1:{port}")
        return
    with socket.socket() as listener:
        try:
            listener.bind(("127.0.0.1", port))
        except OSError as error:
            raise RuntimeError(f"端口 {port} 已被占用，未改动占用者") from error
    token = uuid.uuid4().hex
    env = dict(os.environ, MIHOMOAPP_PREVIEW_TOKEN=token)
    with (RUNTIME / "dev-preview.log").open("ab") as log:
        child = subprocess.Popen(
            ["node", str(VITE), "--host", "127.0.0.1", "--port", str(port), "--strictPort"],
            cwd=APP, env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True,
        )
    info = identity(child.pid)
    if info is None:
        raise RuntimeError("预览启动失败，请查看 .runtime/dev-preview.log")
    state = {"pid": child.pid, "start": info["start"], "token": token, "port": port}
    temporary = STATE.with_suffix(".tmp")
    temporary.write_text(json.dumps(state))
    temporary.chmod(0o600)
    temporary.replace(STATE)
    try:
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        for _ in range(150):
            if child.poll() is not None:
                raise RuntimeError("预览退出，请查看 .runtime/dev-preview.log")
            try:
                with opener.open(f"http://127.0.0.1:{port}", timeout=0.3) as response:
                    if response.status == 200:
                        print(f"预览已就绪：http://127.0.0.1:{port}")
                        return
            except OSError:
                pass
            time.sleep(0.1)
        raise RuntimeError("预览未在期限内就绪，请查看 .runtime/dev-preview.log")
    except BaseException:
        stop(state)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["start", "stop"])
    parser.add_argument("--port", type=int, default=1420)
    args = parser.parse_args()
    if not sys.platform.startswith("linux"):
        parser.error("此进程归属检查使用 Linux /proc；其他平台请直接使用 npm run dev")
    if not 1024 <= args.port <= 65535:
        parser.error("端口必须在 1024–65535 范围内")
    RUNTIME.mkdir(mode=0o700, exist_ok=True)
    with (RUNTIME / "dev-preview.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if args.action == "start":
            start(args.port)
        else:
            state = load_state()
            if state:
                stop(state)
            print("本项目开发预览已停止")


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, OSError) as error:
        print(f"错误：{error}", file=sys.stderr)
        sys.exit(1)
