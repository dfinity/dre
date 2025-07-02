import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

export type DayUTC = string;
export type GetNodeProviderRewardsCalculationResponse = {
    'Ok' : RewardsCalculatorResults
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
export type Percent = number;
export type PrincipalId = Principal;
export interface RewardPeriodArgs { 'start_ts' : bigint, 'end_ts' : bigint }
export interface RewardsCalculatorResults {
  'extrapolated_fr' : Percent,
  'results_by_node' : Array<[NodeId, NodeResults]>,
  'rewards_total' : XDRPermyriad,
}
export type SubnetId = PrincipalId;
export type XDRPermyriad = number;
export interface _SERVICE {
  'get_node_provider_rewards_calculation' : ActorMethod<
    [NodeProviderRewardsCalculationArgs],
    GetNodeProviderRewardsCalculationResponse
  >,
  'get_node_providers_rewards' : ActorMethod<
    [RewardPeriodArgs],
    GetNodeProvidersRewardsResponse
  >,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
