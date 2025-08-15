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
  const NodeMetricsDaily = IDL.Record({
    'day' : DayUTC,
    'subnet_assigned' : SubnetId,
    'original_fr' : Percent,
    'num_blocks_proposed' : IDL.Nat64,
    'subnet_assigned_fr' : Percent,
    'num_blocks_failed' : IDL.Nat64,
    'relative_fr' : Percent,
  });
  const NodeResults = IDL.Record({
    'region' : IDL.Text,
    'avg_extrapolated_fr' : Percent,
    'performance_multiplier' : Percent,
    'node_type' : IDL.Text,
    'base_rewards_per_month' : XDRPermyriad,
    'daily_metrics' : IDL.Vec(NodeMetricsDaily),
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
    'results_by_node' : IDL.Vec(IDL.Tuple(NodeId, NodeResults)),
    'rewards_total' : XDRPermyriad,
  });
  const GetNodeProviderRewardsCalculationResponse = IDL.Variant({
    'Ok' : RewardsCalculatorResults,
    'Err' : IDL.Text,
  });
  const NodeMetricsDailyV1 = IDL.Record({
    'subnet_assigned' : SubnetId,
    'original_fr' : Percent,
    'num_blocks_proposed' : IDL.Nat64,
    'subnet_assigned_fr' : Percent,
    'num_blocks_failed' : IDL.Nat64,
    'relative_fr' : Percent,
  });
  const NodeStatusV1 = IDL.Variant({
    'Unassigned' : IDL.Record({ 'extrapolated_fr' : Percent }),
    'Assigned' : IDL.Record({ 'node_metrics' : NodeMetricsDailyV1 }),
  });
  const DailyResultsV1 = IDL.Record({
    'performance_multiplier' : Percent,
    'node_status' : NodeStatusV1,
    'adjusted_rewards' : XDRPermyriad,
    'base_rewards' : XDRPermyriad,
    'rewards_reduction' : Percent,
  });
  const NodeResultsV1 = IDL.Record({
    'region' : IDL.Text,
    'daily_results' : IDL.Vec(IDL.Tuple(DayUTC, DailyResultsV1)),
    'node_reward_type' : IDL.Text,
    'dc_id' : IDL.Text,
  });
  const RewardsCalculatorResultsV1 = IDL.Record({
    'results_by_node' : IDL.Vec(IDL.Tuple(NodeId, NodeResultsV1)),
    'computation_log' : IDL.Text,
    'rewards_total' : XDRPermyriad,
  });
  const GetNodeProviderRewardsCalculationResponseV1 = IDL.Variant({
    'Ok' : RewardsCalculatorResultsV1,
    'Err' : IDL.Text,
  });
  const NodeProvidersRewards = IDL.Record({
    'rewards_per_provider' : IDL.Vec(IDL.Tuple(PrincipalId, XDRPermyriad)),
  });
  const GetNodeProvidersRewardsResponse = IDL.Variant({
    'Ok' : NodeProvidersRewards,
    'Err' : IDL.Text,
  });
  return IDL.Service({
    'get_node_provider_rewards_calculation' : IDL.Func(
        [NodeProviderRewardsCalculationArgs],
        [GetNodeProviderRewardsCalculationResponse],
        ['query'],
      ),
    'get_node_provider_rewards_calculation_v1' : IDL.Func(
        [NodeProviderRewardsCalculationArgs],
        [GetNodeProviderRewardsCalculationResponseV1],
        ['query'],
      ),
    'get_node_providers_rewards' : IDL.Func(
        [RewardPeriodArgs],
        [GetNodeProvidersRewardsResponse],
        ['query'],
      ),
  });
};
export const init = ({ IDL }) => { return []; };
