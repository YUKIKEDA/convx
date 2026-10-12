"""T2 of the comparison (#598): the scans of the Stanford 3D Scanning Repository.

usage: scans.py <out directory>

Downloads the four models, checks each archive against the SHA-256 recorded
below, and writes the vertices of each reconstructed surface as a point
file: a dimension, a count, and the points, every coordinate as the `f32` of
the model printed so that it reads back to the same value.

The models are not ours and are not committed. They come from the Stanford
Computer Graphics Laboratory, http://graphics.stanford.edu/data/3Dscanrep/ ,
which asks that the source be acknowledged and that the models not be used
commercially without permission. The comparison's document credits it.
"""
import gzip
import hashlib
import io
import os
import struct
import sys
import tarfile
import urllib.request

BASE = 'https://graphics.stanford.edu/pub/3Dscanrep/'

# name, address under BASE, SHA-256 of the download, the member of a tar
# archive that holds the reconstruction (None for a single gzipped file).
MODELS = [
    ('bunny', 'bunny.tar.gz', 'a5720bd96d158df403d153381b8411a727a1d73cff2f33dc9b212d6f75455b84', 'bunny/reconstruction/bun_zipper.ply'),
    ('dragon', 'dragon/dragon_recon.tar.gz', '74ac1d90989c9b1732edee82d57e9ce71452144cf4355f108d8c9c616d28d02f', 'dragon_recon/dragon_vrip.ply'),
    ('buddha', 'happy/happy_recon.tar.gz', '409cd294efbfd8244e15a382b95a9423f153b7776e736c9b09f19ec9d3c10ed0', 'happy_recon/happy_vrip.ply'),
    ('armadillo', 'armadillo/Armadillo.ply.gz', '8b9b56cc36e66d54429b1e1e75bd89e833645bfe0dc7c1afd1205877a7356a3f', None),
]

SIZES = {'char': 1, 'uchar': 1, 'int8': 1, 'uint8': 1, 'short': 2, 'ushort': 2, 'int16': 2, 'uint16': 2,
         'int': 4, 'uint': 4, 'int32': 4, 'uint32': 4, 'float': 4, 'float32': 4, 'double': 8, 'float64': 8}


def vertices(ply):
    """The x, y, z of every vertex of a PLY file, as the floats it stores."""
    stream = io.BytesIO(ply)
    assert stream.readline().strip() == b'ply', 'not a PLY file'
    fmt, count, properties, in_vertex = None, None, [], False
    while True:
        words = stream.readline().split()
        if words[0] == b'end_header':
            break
        if words[0] == b'format':
            fmt = words[1].decode()
        elif words[0] == b'element':
            in_vertex = words[1] == b'vertex'
            if in_vertex:
                count = int(words[2])
        elif words[0] == b'property' and in_vertex:
            assert words[1] != b'list', 'a list among the vertex properties'
            properties.append((words[2].decode(), words[1].decode()))
    names = [name for name, _ in properties]
    assert names[:3] == ['x', 'y', 'z'], names
    assert all(kind in ('float', 'float32') for _, kind in properties[:3]), properties
    points = []
    if fmt == 'ascii':
        for _ in range(count):
            words = stream.readline().split()
            # The text is a decimal of an f32; keep the f32.
            points.append(struct.unpack('<3f', struct.pack('<3f', *(float(w) for w in words[:3]))))
    else:
        order = '<' if fmt == 'binary_little_endian' else '>'
        stride = sum(SIZES[kind] for _, kind in properties)
        data = stream.read(count * stride)
        for i in range(count):
            points.append(struct.unpack_from(order + '3f', data, i * stride))
    assert len(points) == count
    return points


def fetch(address, sha, raw):
    path = os.path.join(raw, os.path.basename(address))
    if not os.path.exists(path):
        with urllib.request.urlopen(BASE + address) as response, open(path, 'wb') as f:
            f.write(response.read())
    data = open(path, 'rb').read()
    found = hashlib.sha256(data).hexdigest()
    if found != sha:
        raise SystemExit(f'{address}: SHA-256 {found}, recorded {sha}')
    return data


def main():
    out = sys.argv[1]
    raw = os.path.join(out, 'raw')
    os.makedirs(raw, exist_ok=True)
    for name, address, sha, member in MODELS:
        data = fetch(address, sha, raw)
        if member is None:
            ply = gzip.decompress(data)
        else:
            with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as tar:
                ply = tar.extractfile(member).read()
        points = vertices(ply)
        path = os.path.join(out, f'{name}-d3-n{len(points)}.txt')
        with open(path, 'w', newline='\n') as f:
            f.write(f'3\n{len(points)}\n')
            for p in points:
                f.write(' '.join(repr(float(x)) for x in p) + '\n')
        print(f'{name} D3 points {len(points)} distinct {len(set(points))}')


if __name__ == '__main__':
    main()
