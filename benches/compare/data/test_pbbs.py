"""Each distribution of pbbs.py against its definition.

Run with `python3 -m unittest` in this directory. The reference is the
closed form of each distribution's radial law (`pbbs.cdf`), which is not how
the points are drawn: the largest gap between the empirical and the exact
distribution of 200,000 radii stays under 0.005 (the 1e-6 quantile of the
Kolmogorov statistic at that count is 0.0061), and the directions average to
the origin.
"""
import math
import unittest

import pbbs

CASES = [('insphere', 2), ('insphere', 3), ('kuzmin', 2), ('plummer', 3)]
COUNT = 200_000


def gap(name, dim, radii):
    radii = sorted(radii)
    n = len(radii)
    worst = 0.0
    for i, r in enumerate(radii):
        exact = pbbs.cdf(name, dim, r)
        worst = max(worst, abs(exact - i / n), abs(exact - (i + 1) / n))
    return worst


class Distributions(unittest.TestCase):
    def test_the_radii_follow_the_definition(self):
        for name, dim in CASES:
            pts = list(pbbs.points(name, dim, COUNT, 598))
            radii = [math.sqrt(sum(x * x for x in p)) for p in pts]
            self.assertLess(gap(name, dim, radii), 0.005, (name, dim))
            # Against another distribution's law the gap is large: the
            # test tells them apart.
            other = 'kuzmin' if name != 'kuzmin' else 'insphere'
            self.assertGreater(gap(other, dim, radii), 0.05, (name, dim))

    def test_the_directions_are_uniform(self):
        for name, dim in CASES:
            pts = list(pbbs.points(name, dim, COUNT, 598))
            mean = [0.0] * dim
            octants = {}
            for p in pts:
                r = math.sqrt(sum(x * x for x in p))
                if r == 0.0:
                    continue
                for a in range(dim):
                    mean[a] += p[a] / r / COUNT
                key = tuple(x > 0.0 for x in p)
                octants[key] = octants.get(key, 0) + 1
            # The mean of unit vectors has a standard deviation of about
            # 1 / sqrt(dim * COUNT) per coordinate, 0.0016 here.
            self.assertLess(max(abs(m) for m in mean), 0.01, (name, dim))
            share = 1.0 / 2 ** dim
            for count in octants.values():
                self.assertLess(abs(count / COUNT - share), 0.01, (name, dim))
            self.assertEqual(len(octants), 2 ** dim, (name, dim))

    def test_the_points_of_a_seed_are_the_same_twice(self):
        a = list(pbbs.points('plummer', 3, 1000, 7))
        b = list(pbbs.points('plummer', 3, 1000, 7))
        c = list(pbbs.points('plummer', 3, 1000, 8))
        self.assertEqual(a, b)
        self.assertNotEqual(a, c)

    def test_the_ball_holds_every_point(self):
        for dim in (2, 3):
            for p in pbbs.points('insphere', dim, 20_000, 3):
                self.assertLessEqual(sum(x * x for x in p), 1.0 + 1e-12)


if __name__ == '__main__':
    unittest.main()
