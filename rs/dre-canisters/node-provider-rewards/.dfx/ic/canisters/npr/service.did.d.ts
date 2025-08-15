import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

export interface DailyResultsV1 {
  'performance_multiplier' : Percent,
  'node_status' : NodeStatusV1,
  'adjusted_rewards' : XDRPermyriad,
  'base_rewards' : XDRPermyriad,
  'rewards_reduction' : Percent,
}
export type DayUTC = string;
export type GetNodeProviderRewardsCalculationResponse = {
    'Ok' : RewardsCalculatorResults
  } |
  { 'Err' : string };
export type GetNodeProviderRewardsCalculationResponseV1 = {
    'Ok' : RewardsCalculatorResultsV1
  } |
  { 'Err' : string };
export type GetNodeProvidersRewardsResponse = { 'Ok' : NodeProvidersRewards } |
  { 'Err' : string };
export type NodeId = PrincipalId;
export interface NodeMetricsDaily {
  'day' : DayUTC,
  'subnet_assigned' : SubnetId,
  'original_fr' : Percent,
  'num_blocks_proposed' : bigint,
  'subnet_assigned_fr' : Percent,
  'num_blocks_failed' : bigint,
  'relative_fr' : Percent,
}
export interface NodeMetricsDailyV1 {
  'subnet_assigned' : SubnetId,
  'original_fr' : Percent,
  'num_blocks_proposed' : bigint,
  'subnet_assigned_fr' : Percent,
  'num_blocks_failed' : bigint,
  'relative_fr' : Percent,
}
export interface NodeProviderRewardsCalculationArgs {
  'provider_id' : PrincipalId,
  'reward_period' : RewardPeriodArgs,
}
export interface NodeProvidersRewards {
  'rewards_per_provider' : Array<[PrincipalId, XDRPermyriad]>,
}
export interface NodeResults {
  'region' : string,
  'avg_extrapolated_fr' : Percent,
  'performance_multiplier' : Percent,
  'node_type' : string,
  'base_rewards_per_month' : XDRPermyriad,
  'daily_metrics' : Array<NodeMetricsDaily>,
  'adjusted_rewards' : XDRPermyriad,
  'rewardable_to' : DayUTC,
  'base_rewards' : XDRPermyriad,
  'avg_relative_fr' : [] | [Percent],
  'rewardable_days' : bigint,
  'rewardable_from' : DayUTC,
  'rewards_reduction' : Percent,
  'dc_id' : string,
}
export interface NodeResultsV1 {
  'region' : string,
  'daily_results' : Array<[DayUTC, DailyResultsV1]>,
  'node_reward_type' : string,
  'dc_id' : string,
}
export type NodeStatusV1 = { 'Unassigned' : { 'extrapolated_fr' : Percent } } |
  { 'Assigned' : { 'node_metrics' : NodeMetricsDailyV1 } };
export type Percent = number;
export type PrincipalId = Principal;
export interface RewardPeriodArgs { 'start_ts' : bigint, 'end_ts' : bigint }
export interface RewardsCalculatorResults {
  'extrapolated_fr' : Percent,
  'results_by_node' : Array<[NodeId, NodeResults]>,
  'rewards_total' : XDRPermyriad,
}
export interface RewardsCalculatorResultsV1 {
  'results_by_node' : Array<[NodeId, NodeResultsV1]>,
  'computation_log' : string,
  'rewards_total' : XDRPermyriad,
}
export type SubnetId = PrincipalId;
export type XDRPermyriad = number;
export interface _SERVICE {
  'get_node_provider_rewards_calculation' : ActorMethod<
    [NodeProviderRewardsCalculationArgs],
    GetNodeProviderRewardsCalculationResponse
  >,
  'get_node_provider_rewards_calculation_v1' : ActorMethod<
    [NodeProviderRewardsCalculationArgs],
    GetNodeProviderRewardsCalculationResponseV1
  >,
  'get_node_providers_rewards' : ActorMethod<
    [RewardPeriodArgs],
    GetNodeProvidersRewardsResponse
  >,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
