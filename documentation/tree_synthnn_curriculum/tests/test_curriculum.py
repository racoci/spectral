import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from synthnn import SynthNN, example_config
from curriculum.spec import STAGES, PARAM_GROUPS
from curriculum.registry import classify, set_trainable_groups


def test_stage_count_and_unique_ids():
    assert len(STAGES) == 22
    assert len({s.id for s in STAGES}) == len(STAGES)
    assert STAGES[0].id == "E00"
    assert STAGES[-1].id == "E21"


def test_parameter_groups_cover_model():
    model = SynthNN(example_config())
    for name, _ in model.named_parameters():
        assert classify(name) in PARAM_GROUPS


def test_freeze_policy():
    model = SynthNN(example_config())
    set_trainable_groups(model, ("pitch",))
    for name, p in model.named_parameters():
        assert p.requires_grad == (classify(name) == "pitch")
