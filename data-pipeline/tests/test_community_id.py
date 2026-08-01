from d3rac_pipeline.community_id import to_community_id, to_community_id_hex


def test_deterministic():
    assert to_community_id("kebbi-river-basin") == to_community_id("kebbi-river-basin")


def test_different_ids_differ():
    assert to_community_id("kebbi-river-basin") != to_community_id("lokoja-confluence")


def test_hex_form_is_32_bytes():
    hex_id = to_community_id_hex("kebbi-river-basin")
    assert hex_id.startswith("0x")
    assert len(hex_id) == 2 + 64  # 0x + 32 bytes as hex
