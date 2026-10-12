"""T1 of the comparison (#598): the point distributions of PBBS.

usage: pbbs.py <insphere|kuzmin|plummer> <dim> <count> <seed> <out file>

The distributions are those of the Problem Based Benchmark Suite
(`testData/geometryData/randPoints.C`, https://github.com/cmuparlay/pbbsbench):

- insphere: uniform in the unit ball, D = 2 or 3.
- kuzmin, D = 2: a uniform direction times r = sqrt(1 / (1 - s)^2 - 1), with
  s uniform in [0, 1). Dense at the center, with a long tail.
- plummer, D = 3: a uniform direction times r = sqrt(s / (1 - s)), with
  s = u^(2/3) and u uniform in [0, 1).

The formulas are PBBS's; the random numbers are not. PBBS hashes the index
of a point, and this draws from Python's Mersenne Twister with the given
seed, so a file made here has the distribution of a PBBS file and not its
points. The file is a dimension, a count, and the points, as rbox writes
them, with every coordinate printed so that it reads back to the same f64.
"""
import math
import random
import sys


def direction(rng, dim):
    """A point uniform on the unit sphere of dimension `dim` (2 or 3)."""
    if dim == 2:
        t = 2.0 * math.pi * rng.random()
        return (math.cos(t), math.sin(t))
    z = 2.0 * rng.random() - 1.0
    t = 2.0 * math.pi * rng.random()
    r = math.sqrt(max(0.0, 1.0 - z * z))
    return (r * math.cos(t), r * math.sin(t), z)


def radius(rng, name, dim):
    u = rng.random()
    if name == 'insphere':
        return u ** (1.0 / dim)
    if name == 'kuzmin' and dim == 2:
        return math.sqrt(1.0 / ((1.0 - u) * (1.0 - u)) - 1.0)
    if name == 'plummer' and dim == 3:
        s = u ** (2.0 / 3.0)
        return math.sqrt(s / (1.0 - s))
    raise SystemExit(f'{name} is not defined for D = {dim}')


def cdf(name, dim, r):
    """The probability that a point's distance from the origin is at most `r`."""
    if name == 'insphere':
        return min(1.0, r ** dim)
    if name == 'kuzmin':
        return 1.0 - 1.0 / math.sqrt(1.0 + r * r)
    return (r * r / (1.0 + r * r)) ** 1.5


def points(name, dim, count, seed):
    rng = random.Random(seed)
    for _ in range(count):
        v = direction(rng, dim)
        r = radius(rng, name, dim)
        yield tuple(r * x for x in v)


def main():
    name, dim, count, seed, out = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
    with open(out, 'w', newline='\n') as f:
        f.write(f'{dim}\n{count}\n')
        for p in points(name, dim, count, seed):
            f.write(' '.join(repr(x) for x in p) + '\n')


if __name__ == '__main__':
    main()
