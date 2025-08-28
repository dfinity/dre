export const idlFactory = ({ IDL }) => {
  const PrincipalId = IDL.Principal;
  const RewardPeriodArgs = IDL.Record({
    'start_ts' : IDL.Nat64,
    'end_ts' : IDL.Nat64,
  });
  const NodeProviderRewardsCalculationArgs = IDL.Record({
    'provider_id' : PrincipalId,
    'reward_period' : RewardPeriodArgs,
  });
  const Percent = IDL.Float64;
  const NodeId = PrincipalId;
  const XDRPermyriad = IDL.Float64;
  const DayUTC = IDL.Text;
  const SubnetId = PrincipalId;
  const NodeMetricsDailyDeprecated = IDL.Record({
    'day' : DayUTC,
    'subnet_assigned' : SubnetId,
    'original_fr' : Percent,
    'num_blocks_proposed' : IDL.Nat64,
    'subnet_assigned_fr' : Percent,
    'num_blocks_failed' : IDL.Nat64,
    'relative_fr' : Percent,
  });
  const NodeResultsDeprecated = IDL.Record({
    'region' : IDL.Text,
    'avg_extrapolated_fr' : Percent,
    'performance_multiplier' : Percent,
    'node_type' : IDL.Text,
    'base_rewards_per_month' : XDRPermyriad,
    'daily_metrics' : IDL.Vec(NodeMetricsDailyDeprecated),
    'adjusted_rewards' : XDRPermyriad,
    'rewardable_to' : DayUTC,
    'base_rewards' : XDRPermyriad,
    'avg_relative_fr' : IDL.Opt(Percent),
    'rewardable_days' : IDL.Nat64,
    'rewardable_from' : DayUTC,
    'rewards_reduction' : Percent,
    'dc_id' : IDL.Text,
  });
  const RewardsCalculatorResults = IDL.Record({
    'extrapolated_fr' : Percent,
    'results_by_node' : IDL.Vec(IDL.Tuple(NodeId, NodeResultsDeprecated)),
    'rewards_total' : XDRPermyriad,
  });
  const GetNodeProviderRewardsCalculationResponseDeprecated = IDL.Variant({
    'Ok' : RewardsCalculatorResults,
    'Err' : IDL.Text,
  });
  const GetNodeProviderRewardsCalculationRequest = IDL.Record({
    'provider_id' : IDL.Principal,
    'historical' : IDL.Bool,
    'to_nanos' : IDL.Nat64,
    'from_nanos' : IDL.Nat64,
  });
  const DayUtc = IDL.Record({ 'value' : IDL.Opt(IDL.Nat64) });
  const Decimal = IDL.Record({ 'human_readable' : IDL.Opt(IDL.Text) });
  const Unassigned = IDL.Record({
    'extrapolated_fr_percent' : IDL.Opt(Decimal),
  });
  const NodeMetricsDaily = IDL.Record({
    'subnet_assigned' : IDL.Opt(IDL.Principal),
    'original_fr_percent' : IDL.Opt(Decimal),
    'num_blocks_proposed' : IDL.Opt(IDL.Nat64),
    'subnet_assigned_fr_percent' : IDL.Opt(Decimal),
    'relative_fr_percent' : IDL.Opt(Decimal),
    'num_blocks_failed' : IDL.Opt(IDL.Nat64),
  });
  const Assigned = IDL.Record({ 'node_metrics' : IDL.Opt(NodeMetricsDaily) });
  const Status = IDL.Variant({
    'Unassigned' : Unassigned,
    'Assigned' : Assigned,
  });
  const NodeStatus = IDL.Record({ 'status' : IDL.Opt(Status) });
  const DailyResults = IDL.Record({
    'day' : IDL.Opt(DayUtc),
    'rewards_reduction_percent' : IDL.Opt(Decimal),
    'node_status' : IDL.Opt(NodeStatus),
    'base_rewards_xdr_permyriad' : IDL.Opt(Decimal),
    'adjusted_rewards_xdr_permyriad' : IDL.Opt(Decimal),
    'performance_multiplier_percent' : IDL.Opt(Decimal),
  });
  const NodeResults = IDL.Record({
    'region' : IDL.Opt(IDL.Text),
    'node_id' : IDL.Opt(IDL.Principal),
    'daily_results' : IDL.Vec(DailyResults),
    'node_reward_type' : IDL.Opt(IDL.Text),
    'dc_id' : IDL.Opt(IDL.Text),
  });
  const BaseRewards = IDL.Record({
    'region' : IDL.Opt(IDL.Text),
    'daily_xdr_permyriad' : IDL.Opt(Decimal),
    'node_reward_type' : IDL.Opt(IDL.Text),
    'monthly_xdr_permyriad' : IDL.Opt(Decimal),
  });
  const DailyBaseRewardsType3 = IDL.Record({
    'day' : IDL.Opt(DayUtc),
    'region' : IDL.Opt(IDL.Text),
    'value_xdr_permyriad' : IDL.Opt(Decimal),
    'nodes_count' : IDL.Opt(IDL.Nat64),
    'avg_coefficient_percent' : IDL.Opt(Decimal),
    'avg_rewards_xdr_permyriad' : IDL.Opt(Decimal),
  });
  const NodeProviderRewards = IDL.Record({
    'rewards_total_xdr_permyriad' : IDL.Opt(IDL.Nat64),
    'nodes_results' : IDL.Vec(NodeResults),
    'base_rewards' : IDL.Vec(BaseRewards),
    'base_rewards_type3' : IDL.Vec(DailyBaseRewardsType3),
  });
  const GetNodeProviderRewardsCalculationResponse = IDL.Variant({
    'Ok' : NodeProviderRewards,
    'Err' : IDL.Text,
  });
  const NodeProvidersRewards = IDL.Record({
    'rewards_per_provider' : IDL.Vec(IDL.Tuple(PrincipalId, XDRPermyriad)),
  });
  const GetNodeProvidersRewardsResponse = IDL.Variant({
    'Ok' : NodeProvidersRewards,
    'Err' : IDL.Text,
  });
  const NodeDailyFR = IDL.Record({
    'node_id' : IDL.Principal,
    'daily_relative_fr' : IDL.Vec(IDL.Tuple(DayUtc, Decimal)),
  });
  const SubnetNodesFR = IDL.Record({
    'subnet_fr' : Decimal,
    'subnet_id' : IDL.Principal,
    'nodes_daily_fr' : IDL.Vec(NodeDailyFR),
  });
  const GetNodesFRBySubnet = IDL.Variant({
    'Ok' : IDL.Vec(SubnetNodesFR),
    'Err' : IDL.Text,
  });
  return IDL.Service({
    'get_node_provider_rewards_calculation' : IDL.Func(
        [NodeProviderRewardsCalculationArgs],
        [GetNodeProviderRewardsCalculationResponseDeprecated],
        ['query'],
      ),
    'get_node_provider_rewards_calculation_v1' : IDL.Func(
        [GetNodeProviderRewardsCalculationRequest],
        [GetNodeProviderRewardsCalculationResponse],
        ['query'],
      ),
    'get_node_providers_rewards' : IDL.Func(
        [RewardPeriodArgs],
        [GetNodeProvidersRewardsResponse],
        ['query'],
      ),
    'get_nodes_fr_by_subnet' : IDL.Func(
        [RewardPeriodArgs],
        [GetNodesFRBySubnet],
        ['query'],
      ),
    'get_subnets_list' : IDL.Func([], [IDL.Vec(SubnetId)], ['query']),
  });
};
export const init = ({ IDL }) => { return []; };
