"""Finite inventory, rotations, resources and identity are pre-result contracts."""
import importlib.util
from pathlib import Path

spec = importlib.util.spec_from_file_location('pipeline_campaign', Path(__file__).with_name('campaign.py'))
campaign = importlib.util.module_from_spec(spec)
spec.loader.exec_module(campaign)


def test_inventory_identity_and_scope():
    p = campaign.protocol()
    assert p['measured_counts'] == dict(supported_paths=216, screen_400k=150,
                                      validation_1_6m=90, private_veneto=12)
    assert sum(p['measured_counts'].values()) == 468
    assert p['baseline_native_sha256'] == '6d8e86aafbe1224e29217ce4565a7c91c22fcc60583bd97b2805b910298ae18d'
    assert p['status'] == 'GATED_NOT_SUBMITTED' and p['candidate_native_sha256'] is None
    assert len(campaign.PROFILES) == 12
    assert p['resources']['queue'] is None and p['resources']['host'] is None
    assert p['rss_guards_gib']['matlab'] == 72
    for calls in p['stages'].values():
        assert all(c['configuration']['rows'] <= 1_600_000 for c in calls)


def test_pairs_warmups_seeds_and_rotation():
    for stage, calls in campaign.protocol()['stages'].items():
        cells = sorted({c['cell'] for c in calls})
        for cell in cells:
            group = [c for c in calls if c['cell'] == cell]
            variants = [c['variant'] for c in group if c['warmup']]
            assert len(variants) == len(set(variants))
            for rep in sorted({c['repetition'] for c in group if not c['warmup']}):
                measured = [c for c in group if not c['warmup'] and c['repetition'] == rep]
                offset = (cell + rep - 1) % len(variants)
                assert [c['variant'] for c in measured] == variants[offset:] + variants[:offset]
                assert len({c['seed'] for c in measured}) == 1
                if stage == 'screen_400k':
                    assert ('matlab' in variants) == (measured[0]['configuration']['threads'] == 28)
