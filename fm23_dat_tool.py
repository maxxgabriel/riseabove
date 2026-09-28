#!/usr/bin/env python3
"""Read-only inspector/extractor for Football Manager 2023 .dat database files.

FM23 database files use a small proprietary ``tad.`` container with embedded
Zstandard frames.  This tool extracts those frames without changing the input.
It intentionally does not repack or write files back into the game directory.
"""

from __future__ import annotations

import argparse
import ctypes
import ctypes.util
import json
import os
import struct
from pathlib import Path


MAGIC = b"\x28\xb5\x2f\xfd"  # Zstandard frame magic
UNKNOWN = (1 << 64) - 1
ERROR = (1 << 64) - 2


class Zstd:
    def __init__(self) -> None:
        candidates = [
            os.environ.get("ZSTD_DLL"),
            r"C:\Program Files\Git\mingw64\bin\libzstd.dll",
            r"C:\Program Files\Git\usr\bin\msys-zstd-1.dll",
            ctypes.util.find_library("zstd"),
        ]
        self.lib = None
        for candidate in candidates:
            if not candidate:
                continue
            try:
                self.lib = ctypes.CDLL(candidate)
                break
            except OSError:
                pass
        if self.lib is None:
            raise RuntimeError(
                "libzstd was not found. Install a Zstandard runtime or set ZSTD_DLL."
            )
        self.lib.ZSTD_getFrameContentSize.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
        self.lib.ZSTD_getFrameContentSize.restype = ctypes.c_ulonglong
        self.lib.ZSTD_findFrameCompressedSize.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
        self.lib.ZSTD_findFrameCompressedSize.restype = ctypes.c_size_t
        self.lib.ZSTD_decompress.argtypes = [
            ctypes.c_void_p,
            ctypes.c_size_t,
            ctypes.c_void_p,
            ctypes.c_size_t,
        ]
        self.lib.ZSTD_decompress.restype = ctypes.c_size_t
        self.lib.ZSTD_isError.argtypes = [ctypes.c_size_t]
        self.lib.ZSTD_isError.restype = ctypes.c_uint
        self.lib.ZSTD_getErrorName.argtypes = [ctypes.c_size_t]
        self.lib.ZSTD_getErrorName.restype = ctypes.c_char_p

    def decompress(self, frame: bytes) -> bytes:
        source = ctypes.create_string_buffer(frame)
        size = self.lib.ZSTD_getFrameContentSize(source, len(frame))
        if size in (UNKNOWN, ERROR):
            raise ValueError("Zstandard frame has no known content size")
        destination = ctypes.create_string_buffer(size)
        result = self.lib.ZSTD_decompress(destination, size, source, len(frame))
        if self.lib.ZSTD_isError(result):
            name = self.lib.ZSTD_getErrorName(result).decode("ascii", "replace")
            raise ValueError(f"Zstandard decompression failed: {name}")
        return destination.raw[:result]

    def compressed_size(self, data: bytes) -> int:
        source = ctypes.create_string_buffer(data)
        size = self.lib.ZSTD_findFrameCompressedSize(source, len(data))
        if self.lib.ZSTD_isError(size):
            name = self.lib.ZSTD_getErrorName(size).decode("ascii", "replace")
            raise ValueError(f"could not find Zstandard frame size: {name}")
        return size


def frame_spans(data: bytes, zstd: Zstd) -> list[tuple[int, int]]:
    """Return exact frame spans, skipping the 4-byte separator between frames."""
    spans = []
    cursor = data.find(MAGIC)
    while cursor >= 0:
        compressed_size = zstd.compressed_size(data[cursor:])
        end = cursor + compressed_size
        spans.append((cursor, end))
        cursor = data.find(MAGIC, end)
    return spans


def inspect(path: Path, zstd: Zstd) -> dict:
    data = path.read_bytes()
    spans = frame_spans(data, zstd)
    if not spans:
        return {
            "file": str(path),
            "bytes": len(data),
            "container_prefix": data[:32].hex(),
            "container_type": "raw/no-zstd-frames",
            "zstd_frame_count": 0,
            "frames": [],
        }
    frames = []
    for index, (start, end) in enumerate(spans):
        frame = data[start:end]
        content_size = zstd.lib.ZSTD_getFrameContentSize(
            ctypes.create_string_buffer(frame), len(frame)
        )
        try:
            decoded = zstd.decompress(frame)
            status = "ok"
        except ValueError as exc:
            decoded = b""
            status = str(exc)
        frames.append(
            {
                "index": index,
                "offset": start,
                "compressed_bytes_candidate": end - start,
                "declared_uncompressed_bytes": (
                    None if content_size in (UNKNOWN, ERROR) else content_size
                ),
                "decoded_bytes": len(decoded),
                "status": status,
            }
        )
    return {
        "file": str(path),
        "bytes": len(data),
        "container_prefix": data[:spans[0][0]].hex() if spans else data[:32].hex(),
        "zstd_frame_count": len(spans),
        "frames": frames,
    }


def extract(path: Path, output: Path, zstd: Zstd, strings_min: int) -> dict:
    data = path.read_bytes()
    spans = frame_spans(data, zstd)
    output.mkdir(parents=True, exist_ok=True)
    manifest = inspect(path, zstd)
    if not spans:
        raw_path = output / "raw.bin"
        raw_path.write_bytes(data)
        manifest["output"] = raw_path.name
        (output / "strings.json").write_text("[]\n", encoding="utf-8")
        (output / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
        return manifest
    strings = []
    for index, (start, end) in enumerate(spans):
        decoded = zstd.decompress(data[start:end])
        block_path = output / f"block_{index:05d}.bin"
        block_path.write_bytes(decoded)
        manifest["frames"][index]["output"] = block_path.name
        for candidate in decoded.replace(b"\x00", b" ").split():
            try:
                text = candidate.decode("utf-8")
            except UnicodeDecodeError:
                continue
            if len(text) >= strings_min and all(32 <= ord(ch) < 127 for ch in text):
                strings.append({"block": index, "text": text})
    (output / "strings.json").write_text(json.dumps(strings, indent=2), encoding="utf-8")
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    inspect_parser = sub.add_parser("inspect", help="show container and frame metadata")
    inspect_parser.add_argument("file", type=Path)
    extract_parser = sub.add_parser("extract", help="decompress frames into an output folder")
    extract_parser.add_argument("file", type=Path)
    extract_parser.add_argument("output", type=Path)
    extract_parser.add_argument("--strings-min", type=int, default=4)
    args = parser.parse_args()
    zstd = Zstd()
    if not args.file.is_file():
        parser.error(f"input does not exist: {args.file}")
    if args.command == "inspect":
        print(json.dumps(inspect(args.file, zstd), indent=2))
    else:
        result = extract(args.file, args.output, zstd, args.strings_min)
        print(json.dumps({k: result[k] for k in ("file", "bytes", "zstd_frame_count")}, indent=2))
        print(f"Extracted blocks to {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
