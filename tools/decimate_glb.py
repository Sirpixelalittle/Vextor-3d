"""Decimate every mesh in a GLB with quadric simplification.

Built for high-poly found models (the 122k-tri pistol) whose stroke
conversions are too dense for a viewmodel: fewer triangles mean fewer
always-drawn crease edges, fewer silhouette strokes, a smaller .vec
embed, and less per-frame CPU. vex-convert only reads POSITION +
indices, so the output carries exactly that (normals/UVs would be
invalidated by decimation anyway). Node transforms, mesh names (the
paint tool keys off them) and materials pass through untouched.

    tools/venv-or-system-python tools/decimate_glb.py IN.glb OUT.glb [reduction]

`reduction` is the fraction of triangles to REMOVE (default 0.82).
Small parts (under FLOOR_TRIS) are reduced at most 50% so trigger-sized
details don't collapse into nothing.

Needs numpy + fast-simplification (pip install fast-simplification).
"""

import json
import struct
import sys
from pathlib import Path

import numpy as np
import fast_simplification

FLOOR_TRIS = 2000
SMALL_PART_MAX_REDUCTION = 0.5

C_F32, C_U16, C_U32 = 5126, 5123, 5125


def read_glb(path):
    d = Path(path).read_bytes()
    assert d[:4] == b"glTF", f"{path} is not a GLB"
    jlen = struct.unpack("<I", d[12:16])[0]
    assert d[16:20] == b"JSON"
    gltf = json.loads(d[20:20 + jlen])
    off = 20 + jlen
    binary = b""
    while off < len(d):
        clen, tag = struct.unpack("<I4s", d[off:off + 8])
        if tag == b"BIN\x00":
            binary = d[off + 8:off + 8 + clen]
        off += 8 + clen
    return gltf, binary


def read_accessor(gltf, binary, index):
    acc = gltf["accessors"][index]
    view = gltf["bufferViews"][acc["bufferView"]]
    start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    comps = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[acc["type"]]
    dtype = {C_F32: np.float32, C_U16: np.uint16, C_U32: np.uint32}[acc["componentType"]]
    item = np.dtype(dtype).itemsize * comps
    stride = view.get("byteStride", item)
    raw = np.frombuffer(binary, dtype=np.uint8)
    n = acc["count"]
    rows = np.lib.stride_tricks.as_strided(
        raw[start:], shape=(n, item), strides=(stride, 1), writeable=False
    ).tobytes()
    return np.frombuffer(rows, dtype=dtype).reshape(n, comps)


def main():
    src, dst = sys.argv[1], sys.argv[2]
    reduction = float(sys.argv[3]) if len(sys.argv) > 3 else 0.82
    gltf, binary = read_glb(src)

    out_bin = bytearray()
    accessors, views, meshes = [], [], []
    total_in = total_out = 0

    def push(arr, target):
        arr = np.ascontiguousarray(arr)
        views.append({
            "buffer": 0,
            "byteOffset": len(out_bin),
            "byteLength": arr.nbytes,
            "target": target,
        })
        out_bin.extend(arr.tobytes())
        out_bin.extend(b"\x00" * (-len(out_bin) % 4))
        return len(views) - 1

    for mesh in gltf["meshes"]:
        prims_out = []
        for prim in mesh["primitives"]:
            pos = read_accessor(gltf, binary, prim["attributes"]["POSITION"]).astype(np.float32)
            idx = read_accessor(gltf, binary, prim["indices"]).astype(np.uint32).reshape(-1, 3)
            r = reduction if len(idx) >= FLOOR_TRIS else min(reduction, SMALL_PART_MAX_REDUCTION)
            new_pos, new_idx = fast_simplification.simplify(pos, idx, target_reduction=r)
            new_pos = np.asarray(new_pos, dtype=np.float32)
            new_idx = np.asarray(new_idx, dtype=np.uint32)
            total_in += len(idx)
            total_out += len(new_idx)

            pv = push(new_pos, 34962)
            accessors.append({
                "bufferView": pv, "componentType": C_F32, "count": len(new_pos),
                "type": "VEC3",
                "min": [float(v) for v in new_pos.min(axis=0)],
                "max": [float(v) for v in new_pos.max(axis=0)],
            })
            pa = len(accessors) - 1
            iv = push(new_idx.reshape(-1), 34963)
            accessors.append({
                "bufferView": iv, "componentType": C_U32,
                "count": int(new_idx.size), "type": "SCALAR",
            })
            prim_out = {"attributes": {"POSITION": pa}, "indices": len(accessors) - 1}
            if "material" in prim:
                prim_out["material"] = prim["material"]
            prims_out.append(prim_out)
        meshes.append({"name": mesh.get("name", ""), "primitives": prims_out})

    out = {k: v for k, v in gltf.items()
           if k in ("asset", "scene", "scenes", "nodes", "materials", "extensionsUsed")}
    out.update(meshes=meshes, accessors=accessors, bufferViews=views,
               buffers=[{"byteLength": len(out_bin)}])

    body = json.dumps(out, separators=(",", ":")).encode()
    body += b" " * (-len(body) % 4)
    blob = bytearray()
    blob += b"glTF" + struct.pack("<II", 2, 12 + 8 + len(body) + 8 + len(out_bin))
    blob += struct.pack("<I", len(body)) + b"JSON" + body
    blob += struct.pack("<I", len(out_bin)) + b"BIN\x00" + out_bin
    Path(dst).write_bytes(bytes(blob))
    print(f"{total_in} -> {total_out} tris ({total_out / max(total_in, 1):.0%}), "
          f"{len(blob) / 1e6:.1f} MB -> {dst}")


if __name__ == "__main__":
    main()
