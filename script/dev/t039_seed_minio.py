#!/usr/bin/env python3
"""Seed a local MinIO (S3-compatible) stand for T039 live verification.

Creates the demo buckets and objects the vision grims and the
`live_minio_chunked_transfer_byte_proof` services test expect
(`photos`, `documents`). Requires boto3. Run after:

    podman run -d --name minio -p 9000:9000 -p 9001:9001 \\
        -e MINIO_ROOT_USER=minioadmin -e MINIO_ROOT_PASSWORD=minioadmin \\
        quay.io/minio/minio server /data --console-address ':9001'
"""
import boto3
from botocore.config import Config

s3 = boto3.client(
    "s3",
    endpoint_url="http://127.0.0.1:9000",
    aws_access_key_id="minioadmin",
    aws_secret_access_key="minioadmin",
    region_name="us-east-1",
    config=Config(s3={"addressing_style": "path"}),
)

for b in ["photos", "documents"]:
    try:
        s3.create_bucket(Bucket=b)
        print(f"created bucket {b}")
    except Exception as e:
        print(f"bucket {b}: {e}")

objects = {
    "photos": {
        "wallpaper.png": bytes(range(256)) * 128,  # 32 KiB binary blob
        "holiday/beach.jpg": b"jpeg-bytes-" * 512,
        "holiday/mountains.jpg": b"jpeg-bytes-" * 768,
        "family/portrait.jpg": b"jpeg-bytes-" * 1024,
        "notes.txt": b"hello from local minio\n" * 200,
    },
    "documents": {
        "readme.md": b"# Chronos-FM\n\nT039 live S3 verification.\n" * 40,
        "archive.tar.gz": bytes(range(256)) * 4096,  # 1 MiB
    },
}
for bucket, items in objects.items():
    for key, body in items.items():
        s3.put_object(Bucket=bucket, Key=key, Body=body)
        print(f"  put {bucket}/{key} ({len(body)} bytes)")

for b in ["photos", "documents"]:
    resp = s3.list_objects_v2(Bucket=b)
    print(f"{b}: {[o['Key'] for o in resp.get('Contents', [])]}")
