import importlib.util
from pathlib import Path
import sys
import pytest

HERE=Path(__file__).parent
sys.path.insert(0,str(HERE))
from screen_runner import command,read_results
from screen_validate import compare,numeric_gap,performance


def test_commands_are_complete_and_inference_populations_are_not_widened():
    from campaign import PROFILES
    assert len(PROFILES)==12
    for name,p in PROFILES.items():
        text=command(name,104729)
        assert 'tolerance(' not in text and 'memory_gib(' not in text
        assert 'seed(104729)' in text and 'batch(auto)' in text
        if p['family']=='component_inference':
            assert 'stayers(movers)' in text and 'inferencegramprobes(513)' in text
        if p['family']=='exact': assert p['rows']==800 and p['fixture']['firms']==20


def test_read_rejects_duplicate_and_partial_rows(tmp_path):
    file=tmp_path/'result.tsv'
    header='kind\tname\trow\tcolumn\tvalue\n'
    row='scalar\tprobes\t0\t0\t200\n'
    file.write_text(header+row+row)
    with pytest.raises(ValueError,match='duplicate'): read_results(file)
    file.write_text(header+'scalar\tprobes\t0\n')
    with pytest.raises(ValueError,match='partial'): read_results(file)


def matrix(name,number):
    return {('matrix',name,1,1):str(number),('rownames',name,0,0):'a',('colnames',name,0,0):'a'}


def test_covariance_intervals_availability_and_common_draw_gate():
    left=matrix('results',1)|matrix('V',.5)|matrix('component_inference',.7)
    assert compare(left,left)['V']==0
    for name in ('V','component_inference'):
        right=left|{('matrix',name,1,1):'9'}
        with pytest.raises(ValueError,match='statistical gate'): compare(left,right)
    point=matrix('results',1)|{('matrix','results',3,1):'1'}
    with pytest.raises(ValueError,match='statistical gate'):
        compare(point,point|{('matrix','results',3,1):'9'})
    with pytest.raises(ValueError,match='availability'):
        numeric_gap('.a','.b')
    with pytest.raises(ValueError,match='availability'):
        numeric_gap('.','.123')
    with pytest.raises(ValueError,match='availability'):
        compare(left,left|{('macro','inference_support_status',0,0):'withheld'})
    with pytest.raises(ValueError,match='shape'):
        compare(left,{k:v for k,v in left.items() if k[1]!='V'})


def test_performance_includes_inference_and_each_thread_cell():
    pairs=[]
    for name,threads,ratio in [('mover_match_both',1,.8),('mover_match_both',7,.8),
        ('structured_observation_inference',1,1.10)]:
        pairs.extend(dict(profile=name,threads=threads,ratio=ratio) for _ in range(3))
    receipt=performance(pairs)
    assert not receipt['pass_gate']
    assert receipt['profile_thread_median_candidate_over_baseline']['structured_observation_inference/T1']==1.1
    assert 'family:component_inference' in receipt['geometric_candidate_over_baseline']
