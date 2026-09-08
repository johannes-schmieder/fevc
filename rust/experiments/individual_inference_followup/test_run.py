import gzip
import json
import pytest
from run import Q1_CELLS, load_cells, same, sha, write


@pytest.fixture
def evidence(tmp_path):
    (tmp_path/'source.tar.gz').write_bytes(b'synthetic test bundle')
    manifest = {'cells': [], 'tasks': [], 'binaries': {'match_q0': 'binary', 'match_q1': 'binary', 'observation': 'binary'},
                'bundle_sha256': sha(tmp_path/'source.tar.gz')}
    keys = [('match_q0', f'cell{i}', 20) for i in range(15)]+sorted(Q1_CELLS)
    for family, cell, k in keys:
        manifest['cells'].append({'family': family, 'cell': cell, 'k': k, 'reference': 'q0' if family == 'match_q0' else 'q1'})
        task = {'id': f'{family}-{cell}', 'family': family, 'cell': cell, 'k': k, 'start': 0, 'reps': 400, 'master': 7}
        manifest['tasks'].append(task)
        folder = tmp_path/'tasks'/task['id']; folder.mkdir(parents=True)
        rows = [{'kind': 'task'}, {'kind': 'design', 'truth': [1., 2., 3., 4.]}]
        rows += [{'kind': 'call', 'replication': i} for i in range(400)]
        (folder/'rows.jsonl.gz').write_bytes(gzip.compress('\n'.join(map(json.dumps, rows)).encode()))
        write(folder/'receipt.json', {'task': task, 'output_sha256': sha(folder/'rows.jsonl.gz'), 'binary_sha256': 'binary'})
    write(tmp_path/'manifest.json', manifest)
    write(tmp_path/'result.json', {'manifest_sha256': sha(tmp_path/'manifest.json')})
    return tmp_path


def test_complete_inventory(evidence):
    _, groups = load_cells(evidence)
    assert len(groups) == 17 and sum(len(e['calls']) for e in groups.values()) == 6800


@pytest.mark.parametrize('fault', ['missing', 'duplicate', 'malformed', 'changed_hash'])
def test_rejects_bad_inventory(evidence, fault):
    folder = next((evidence/'tasks').iterdir()); path = folder/'rows.jsonl.gz'
    rows = [json.loads(line) for line in gzip.open(path, 'rt')]
    if fault == 'missing':
        rows.pop()
    elif fault == 'duplicate':
        rows[-1]['replication'] = rows[-2]['replication']
    else:
        rows[-1]['kind'] = 'incorrect'
    path.write_bytes(gzip.compress('\n'.join(map(json.dumps, rows)).encode()))
    if fault != 'changed_hash':
        receipt = json.loads((folder/'receipt.json').read_text()); receipt['output_sha256'] = sha(path)
        (folder/'receipt.json').write_text(json.dumps(receipt))
    with pytest.raises(ValueError):
        load_cells(evidence)


def test_comparison_does_not_round_integer_rng_keys():
    assert not same(17700346200288709309, 17700346200288709308)
    assert not same([1.], [1., 2.])
    assert not same({'x': 1.}, {'y': 1.})
