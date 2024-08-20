from unittest.mock import MagicMock, patch
import pytest
from governance import GovernanceCanister

DATA = [
    {
        'Ok': {
            'dissolve_delay_seconds': 252460800,
            'recent_ballots': [
                {'vote': 1, 'proposal_id': [{'id': 131751}]},
                {'vote': 1, 'proposal_id': [{'id': 131515}]},
                {'vote': 1, 'proposal_id': [{'id': 131514}]},
                {'vote': 1, 'proposal_id': [{'id': 131513}]},
                {'vote': 1, 'proposal_id': [{'id': 131512}]},
                {'vote': 1, 'proposal_id': [{'id': 131507}]},
                {'vote': 1, 'proposal_id': [{'id': 131508}]},
                {'vote': 1, 'proposal_id': [{'id': 131511}]}
            ],
            'neuron_type': [],
            'created_timestamp_seconds': 1620662400,
            'state': 1,
            'stake_e8s': 1000000000,
            'joined_community_fund_timestamp_seconds': [],
            'retrieved_at_timestamp_seconds': 1723735594,
            'visibility': [],
            'known_neuron_data': [],
            'voting_power': 2408274052,
            'age_seconds': 103073194
        }
    }
]

@pytest.fixture
@patch('ic.Canister', autospec=True)
def canister_mock(MockCanister):
    canister = MagicMock()
    canister.get_neuron_info.return_value = DATA
    MockCanister.return_value = canister
    return canister

def test_dfinity_has_voted():
    governance_canister = GovernanceCanister()
    result = governance_canister.has_dfinity_voted(131515)
    assert result == True

def test_dfinity_has_not_voted():
    governance_canister = GovernanceCanister()
    result = governance_canister.has_dfinity_voted(0)
    assert result == False
