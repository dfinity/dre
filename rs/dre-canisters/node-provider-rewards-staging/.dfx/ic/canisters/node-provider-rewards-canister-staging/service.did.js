export const idlFactory = ({ IDL }) => {
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
  const DateUtc = IDL.Record({
    'day' : IDL.Opt(IDL.Nat32),
    'month' : IDL.Opt(IDL.Nat32),
    'year' : IDL.Opt(IDL.Nat32),
  });
  const GetNodeProvidersRewardsRequest = IDL.Record({
    'to_day' : DateUtc,
    'from_day' : DateUtc,
  });
  const NodeProvidersRewards = IDL.Record({
    'rewards_xdr_permyriad' : IDL.Vec(IDL.Tuple(IDL.Principal, IDL.Nat64)),
  });
  const GetNodeProvidersRewardsResponse = IDL.Variant({
    'Ok' : NodeProvidersRewards,
    'Err' : IDL.Text,
  });
  const GetNodeProvidersRewardsCalculationRequest = IDL.Record({
    'day' : DateUtc,
  });
  const NodeTypeRegionBaseRewards = IDL.Record({
    'region' : IDL.Opt(IDL.Text),
    'daily_xdr_permyriad' : IDL.Opt(IDL.Nat64),
    'node_reward_type' : IDL.Opt(IDL.Text),
    'monthly_xdr_permyriad' : IDL.Opt(IDL.Nat64),
  });
  const NodeMetricsDaily = IDL.Record({
    'subnet_assigned' : IDL.Opt(IDL.Principal),
    'original_fr_percent' : IDL.Opt(IDL.Float64),
    'num_blocks_proposed' : IDL.Opt(IDL.Nat64),
    'subnet_assigned_fr_percent' : IDL.Opt(IDL.Float64),
    'relative_fr_percent' : IDL.Opt(IDL.Float64),
    'num_blocks_failed' : IDL.Opt(IDL.Nat64),
  });
  const DailyNodeFailureRate = IDL.Variant({
    'SubnetMember' : IDL.Record({ 'node_metrics' : IDL.Opt(NodeMetricsDaily) }),
    'NonSubnetMember' : IDL.Record({
      'extrapolated_fr_percent' : IDL.Opt(IDL.Float64),
    }),
  });
  const DailyNodeRewards = IDL.Record({
    'region' : IDL.Opt(IDL.Text),
    'rewards_reduction_percent' : IDL.Opt(IDL.Float64),
    'node_id' : IDL.Opt(IDL.Principal),
    'daily_node_fr' : IDL.Opt(DailyNodeFailureRate),
    'base_rewards_xdr_permyriad' : IDL.Opt(IDL.Nat64),
    'node_reward_type' : IDL.Opt(IDL.Text),
    'adjusted_rewards_xdr_permyriad' : IDL.Opt(IDL.Nat64),
    'performance_multiplier_percent' : IDL.Opt(IDL.Float64),
    'dc_id' : IDL.Opt(IDL.Text),
  });
  const Type3BaseRewards = IDL.Record({
    'region' : IDL.Opt(IDL.Text),
    'daily_xdr_permyriad' : IDL.Opt(IDL.Nat64),
    'nodes_count' : IDL.Opt(IDL.Nat64),
    'avg_coefficient_percent' : IDL.Opt(IDL.Float64),
    'avg_rewards_xdr_permyriad' : IDL.Opt(IDL.Nat64),
  });
  const DailyNodeProviderRewards = IDL.Record({
    'rewards_total_xdr_permyriad' : IDL.Opt(IDL.Nat64),
    'base_rewards' : IDL.Vec(NodeTypeRegionBaseRewards),
    'daily_nodes_rewards' : IDL.Vec(DailyNodeRewards),
    'base_rewards_type3' : IDL.Vec(Type3BaseRewards),
  });
  const DailyResults = IDL.Record({
    'subnets_fr' : IDL.Vec(IDL.Tuple(IDL.Principal, IDL.Float64)),
    'provider_results' : IDL.Vec(
      IDL.Tuple(IDL.Principal, DailyNodeProviderRewards)
    ),
  });
  const GetNodeProvidersRewardsCalculationResponse = IDL.Variant({
    'Ok' : DailyResults,
    'Err' : IDL.Text,
  });
  return IDL.Service({
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
    'get_node_providers_rewards_calculation' : IDL.Func(
        [GetNodeProvidersRewardsCalculationRequest],
        [GetNodeProvidersRewardsCalculationResponse],
        ['query'],
      ),
  });
};
export const init = ({ IDL }) => { return []; };
