import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

export interface Assigned { 'node_metrics' : [] | [NodeMetricsDaily] }
export interface BaseRewards {
  'region' : [] | [string],
  'daily_xdr_permyriad' : [] | [Decimal],
  'node_reward_type' : [] | [string],
  'monthly_xdr_permyriad' : [] | [Decimal],
}
export interface DailyBaseRewardsType3 {
  'day' : [] | [DayUtc],
  'region' : [] | [string],
  'value_xdr_permyriad' : [] | [Decimal],
  'nodes_count' : [] | [bigint],
  'avg_coefficient_percent' : [] | [Decimal],
  'avg_rewards_xdr_permyriad' : [] | [Decimal],
}
export interface DailyResults {
  'day' : [] | [DayUtc],
  'rewards_reduction_percent' : [] | [Decimal],
  'node_status' : [] | [NodeStatus],
  'base_rewards_xdr_permyriad' : [] | [Decimal],
  'adjusted_rewards_xdr_permyriad' : [] | [Decimal],
  'performance_multiplier_percent' : [] | [Decimal],
}
export type DayUTC = string;
export interface DayUtc { 'value' : [] | [bigint] }
export interface Decimal { 'human_readable' : [] | [string] }
export interface GetNodeProviderRewardsCalculationRequest {
  'provider_id' : Principal,
  'historical' : boolean,
  'to_nanos' : bigint,
  'from_nanos' : bigint,
}
export type GetNodeProviderRewardsCalculationResponse = {
    'Ok' : NodeProviderRewards
  } |
  { 'Err' : string };
export type GetNodeProviderRewardsCalculationResponseDeprecated = {
    'Ok' : RewardsCalculatorResults
  } |
  { 'Err' : string };
export type GetNodeProvidersRewardsResponse = { 'Ok' : NodeProvidersRewards } |
  { 'Err' : string };
export type GetNodesFRBySubnet = { 'Ok' : Array<SubnetNodesFR> } |
  { 'Err' : string };
export interface NodeDailyFR {
  'node_id' : Principal,
  'daily_relative_fr' : Array<[DayUtc, Decimal]>,
}
export type NodeId = PrincipalId;
export interface NodeMetricsDaily {
  'subnet_assigned' : [] | [Principal],
  'original_fr_percent' : [] | [Decimal],
  'num_blocks_proposed' : [] | [bigint],
  'subnet_assigned_fr_percent' : [] | [Decimal],
  'relative_fr_percent' : [] | [Decimal],
  'num_blocks_failed' : [] | [bigint],
}
export interface NodeMetricsDailyDeprecated {
  'day' : DayUTC,
  'subnet_assigned' : SubnetId,
  'original_fr' : Percent,
  'num_blocks_proposed' : bigint,
  'subnet_assigned_fr' : Percent,
  'num_blocks_failed' : bigint,
  'relative_fr' : Percent,
}
export interface NodeProviderRewards {
  'rewards_total_xdr_permyriad' : [] | [bigint],
  'nodes_results' : Array<NodeResults>,
  'base_rewards' : Array<BaseRewards>,
  'base_rewards_type3' : Array<DailyBaseRewardsType3>,
}
export interface NodeProviderRewardsCalculationArgs {
  'provider_id' : PrincipalId,
  'reward_period' : RewardPeriodArgs,
}
export interface NodeProvidersRewards {
  'rewards_per_provider' : Array<[PrincipalId, XDRPermyriad]>,
}
export interface NodeResults {
  'region' : [] | [string],
  'node_id' : [] | [Principal],
  'daily_results' : Array<DailyResults>,
  'node_reward_type' : [] | [string],
  'dc_id' : [] | [string],
}
export interface NodeResultsDeprecated {
  'region' : string,
  'avg_extrapolated_fr' : Percent,
  'performance_multiplier' : Percent,
  'node_type' : string,
  'base_rewards_per_month' : XDRPermyriad,
  'daily_metrics' : Array<NodeMetricsDailyDeprecated>,
  'adjusted_rewards' : XDRPermyriad,
  'rewardable_to' : DayUTC,
  'base_rewards' : XDRPermyriad,
  'avg_relative_fr' : [] | [Percent],
  'rewardable_days' : bigint,
  'rewardable_from' : DayUTC,
  'rewards_reduction' : Percent,
  'dc_id' : string,
}
export interface NodeStatus { 'status' : [] | [Status] }
export type Percent = number;
export type PrincipalId = Principal;
export interface RewardPeriodArgs { 'start_ts' : bigint, 'end_ts' : bigint }
export interface RewardsCalculatorResults {
  'extrapolated_fr' : Percent,
  'results_by_node' : Array<[NodeId, NodeResultsDeprecated]>,
  'rewards_total' : XDRPermyriad,
}
export type Status = { 'Unassigned' : Unassigned } |
  { 'Assigned' : Assigned };
export type SubnetId = PrincipalId;
export interface SubnetNodesFR {
  'subnet_fr' : Decimal,
  'subnet_id' : Principal,
  'nodes_daily_fr' : Array<NodeDailyFR>,
}
export interface Unassigned { 'extrapolated_fr_percent' : [] | [Decimal] }
export type XDRPermyriad = number;
export interface _SERVICE {
  'get_node_provider_rewards_calculation' : ActorMethod<
    [NodeProviderRewardsCalculationArgs],
    GetNodeProviderRewardsCalculationResponseDeprecated
  >,
  'get_node_provider_rewards_calculation_v1' : ActorMethod<
    [GetNodeProviderRewardsCalculationRequest],
    GetNodeProviderRewardsCalculationResponse
  >,
  'get_node_providers_rewards' : ActorMethod<
    [RewardPeriodArgs],
    GetNodeProvidersRewardsResponse
  >,
  'get_nodes_fr_by_subnet' : ActorMethod<
    [RewardPeriodArgs],
    GetNodesFRBySubnet
  >,
  'get_subnets_list' : ActorMethod<[], Array<SubnetId>>,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
