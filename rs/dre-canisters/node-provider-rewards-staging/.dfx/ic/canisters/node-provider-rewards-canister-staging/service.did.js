export const idlFactory = ({ IDL }) => {
  const GetNodeProviderRewardsCalculationRequest = IDL.Record({
    'from_day_timestamp_nanos' : IDL.Nat64,
    'to_day_timestamp_nanos' : IDL.Nat64,
    'provider_id' : IDL.Principal,
  });
  const Decimal = IDL.Record({ 'human_readable' : IDL.Opt(IDL.Text) });
  const NodeMetricsDaily = IDL.Record({
    'subnet_assigned' : IDL.Opt(IDL.Principal),
    'original_fr_percent' : IDL.Opt(Decimal),
    'num_blocks_proposed' : IDL.Opt(IDL.Nat64),
    'subnet_assigned_fr_percent' : IDL.Opt(Decimal),
    'relative_fr_percent' : IDL.Opt(Decimal),
    'num_blocks_failed' : IDL.Opt(IDL.Nat64),
  });
  const NodeStatus = IDL.Variant({
    'Unassigned' : IDL.Record({ 'extrapolated_fr_percent' : IDL.Opt(Decimal) }),
    'Assigned' : IDL.Record({ 'node_metrics' : IDL.Opt(NodeMetricsDaily) }),
  });
  const NodeResults = IDL.Record({
    'region' : IDL.Opt(IDL.Text),
    'rewards_reduction_percent' : IDL.Opt(Decimal),
    'node_id' : IDL.Opt(IDL.Principal),
    'node_status' : IDL.Opt(NodeStatus),
    'base_rewards_xdr_permyriad' : IDL.Opt(Decimal),
    'node_reward_type' : IDL.Opt(IDL.Text),
    'adjusted_rewards_xdr_permyriad' : IDL.Opt(Decimal),
    'performance_multiplier_percent' : IDL.Opt(Decimal),
    'dc_id' : IDL.Opt(IDL.Text),
  });
  const BaseRewards = IDL.Record({
    'region' : IDL.Opt(IDL.Text),
    'daily_xdr_permyriad' : IDL.Opt(Decimal),
    'node_reward_type' : IDL.Opt(IDL.Text),
    'monthly_xdr_permyriad' : IDL.Opt(Decimal),
  });
  const BaseRewardsType3 = IDL.Record({
    'region' : IDL.Opt(IDL.Text),
    'value_xdr_permyriad' : IDL.Opt(Decimal),
    'nodes_count' : IDL.Opt(IDL.Nat64),
    'avg_coefficient_percent' : IDL.Opt(Decimal),
    'avg_rewards_xdr_permyriad' : IDL.Opt(Decimal),
  });
  const NodeProviderRewards = IDL.Record({
    'rewards_total_xdr_permyriad' : IDL.Opt(Decimal),
    'nodes_results' : IDL.Vec(NodeResults),
    'base_rewards' : IDL.Vec(BaseRewards),
    'base_rewards_type3' : IDL.Vec(BaseRewardsType3),
  });
  const DayUtc = IDL.Record({ 'value' : IDL.Opt(IDL.Nat64) });
  const NodeProviderRewardsDaily = IDL.Record({
    'node_provider_rewards' : IDL.Opt(NodeProviderRewards),
    'day_utc' : IDL.Opt(DayUtc),
  });
  const GetNodeProviderRewardsCalculationResponse = IDL.Variant({
    'Ok' : IDL.Vec(NodeProviderRewardsDaily),
    'Err' : IDL.Text,
  });
  const GetNodeProvidersMonthlyXdrRewardsRequest = IDL.Record({
    'registry_version' : IDL.Opt(IDL.Nat64),
  });
  const NodeProvidersMonthlyXdrRewards = IDL.Record({
    'registry_version' : IDL.Opt(IDL.Nat64),
    'rewards' : IDL.Vec(IDL.Tuple(IDL.Principal, IDL.Nat64)),
  });
  const GetNodeProvidersMonthlyXdrRewardsResponse = IDL.Record({
    'error' : IDL.Opt(IDL.Text),
    'rewards' : IDL.Opt(NodeProvidersMonthlyXdrRewards),
  });
  const GetNodeProvidersRewardsRequest = IDL.Record({
    'from_day_timestamp_nanos' : IDL.Nat64,
    'to_day_timestamp_nanos' : IDL.Nat64,
  });
  const NodeProvidersRewards = IDL.Record({
    'rewards_xdr_permyriad' : IDL.Vec(IDL.Tuple(IDL.Principal, IDL.Nat64)),
  });
  const GetNodeProvidersRewardsResponse = IDL.Variant({
    'Ok' : NodeProvidersRewards,
    'Err' : IDL.Text,
  });
  return IDL.Service({
    'get_node_provider_rewards_calculation' : IDL.Func(
        [GetNodeProviderRewardsCalculationRequest],
        [GetNodeProviderRewardsCalculationResponse],
        ['query'],
      ),
    'get_node_providers_monthly_xdr_rewards' : IDL.Func(
        [GetNodeProvidersMonthlyXdrRewardsRequest],
        [GetNodeProvidersMonthlyXdrRewardsResponse],
        [],
      ),
    'get_node_providers_rewards' : IDL.Func(
        [GetNodeProvidersRewardsRequest],
        [GetNodeProvidersRewardsResponse],
        [],
      ),
  });
};
export const init = ({ IDL }) => { return []; };
