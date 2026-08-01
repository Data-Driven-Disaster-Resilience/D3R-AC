from d3rac_pipeline.fixed_point import SCALE, from_fixed_point, to_fixed_point


def test_zero_and_one():
    assert to_fixed_point(0.0) == 0
    assert to_fixed_point(1.0) == SCALE


def test_known_value_matches_readme_example():
    # docs/data-pipeline-srs.md / RiskRegistry.sol constructor comment:
    # 0.35 * 1e18 = 350000000000000000
    assert to_fixed_point(0.35) == 350_000_000_000_000_000


def test_round_half_up():
    # 0.125 at 1e18 scale -> 125000000000000000 exactly, no rounding needed
    assert to_fixed_point(0.125) == 125_000_000_000_000_000


def test_clamps_out_of_range_low():
    assert to_fixed_point(-0.5) == 0


def test_clamps_out_of_range_high():
    assert to_fixed_point(1.5) == SCALE


def test_clamps_floating_point_noise_above_one():
    # e.g. 0.1 + 0.9 can produce 1.0000000000000002 in float arithmetic
    assert to_fixed_point(1.0000000002) == SCALE


def test_round_trip():
    original = 0.618
    fp = to_fixed_point(original)
    assert abs(from_fixed_point(fp) - original) < 1e-9


def test_never_exceeds_scale():
    for v in [0.0, 0.1, 0.5, 0.9999999, 1.0]:
        assert 0 <= to_fixed_point(v) <= SCALE
