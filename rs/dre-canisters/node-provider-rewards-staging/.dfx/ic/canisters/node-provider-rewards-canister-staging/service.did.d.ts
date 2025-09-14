import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

export interface BaseRewards {
  'region' : [] | [string],
  'daily_xdr_permyriad' : [] | [Decimal],
  'node_reward_type' : [] | [string],
  'monthly_xdr_permyriad' : [] | [Decimal],
}
export interface BaseRewardsType3 {
  'region' : [] | [string],
  'value_xdr_permyriad' : [] | [Decimal],
  'nodes_count' : [] | [bigint],
  'avg_coefficient_percent' : [] | [Decimal],
  'avg_rewards_xdr_permyriad' : [] | [Decimal],
}
export interface DayUtc { 'value' : [] | [bigint] }
export interface Decimal { 'human_readable' : [] | [string] }
export interface GetNodeProviderRewardsCalculationRequest {
  'from_day_timestamp_nanos' : bigint,
  'to_day_timestamp_nanos' : bigint,
  'provider_id' : Principal,
}
export type GetNodeProviderRewardsCalculationResponse = {
    'Ok' : Array<NodeProviderRewardsDaily>
  } |
  { 'Err' : string };
export interface GetNodeProvidersMonthlyXdrRewardsRequest {
  'registry_version' : [] | [bigint],
}
export interface GetNodeProvidersMonthlyXdrRewardsResponse {
  'error' : [] | [string],
  'rewards' : [] | [NodeProvidersMonthlyXdrRewards],
}
export interface GetNodeProvidersRewardsRequest {
  'from_day_timestamp_nanos' : bigint,
  'to_day_timestamp_nanos' : bigint,
}
export type GetNodeProvidersRewardsResponse = { 'Ok' : NodeProvidersRewards } |
  { 'Err' : string };
export interface NodeMetricsDaily {
  'subnet_assigned' : [] | [Principal],
  'original_fr_percent' : [] | [Decimal],
  'num_blocks_proposed' : [] | [bigint],
  'subnet_assigned_fr_percent' : [] | [Decimal],
  'relative_fr_percent' : [] | [Decimal],
  'num_blocks_failed' : [] | [bigint],
}
export interface NodeProviderRewards {
  'rewards_total_xdr_permyriad' : [] | [Decimal],
  'nodes_results' : Array<NodeResults>,
  'base_rewards' : Array<BaseRewards>,
  'base_rewards_type3' : Array<BaseRewardsType3>,
}
export interface NodeProviderRewardsDaily {
  'node_provider_rewards' : [] | [NodeProviderRewards],
  'day_utc' : [] | [DayUtc],
}
export interface NodeProvidersMonthlyXdrRewards {
  'registry_version' : [] | [bigint],
  'rewards' : Array<[Principal, bigint]>,
}
export interface NodeProvidersRewards {
  'rewards_xdr_permyriad' : Array<[Principal, bigint]>,
}
export interface NodeResults {
  'region' : [] | [string],
  'rewards_reduction_percent' : [] | [Decimal],
  'node_id' : [] | [Principal],
  'node_status' : [] | [NodeStatus],
  'base_rewards_xdr_permyriad' : [] | [Decimal],
  'node_reward_type' : [] | [string],
  'adjusted_rewards_xdr_permyriad' : [] | [Decimal],
  'performance_multiplier_percent' : [] | [Decimal],
  'dc_id' : [] | [string],
}
export type NodeStatus = {
    'Unassigned' : { 'extrapolated_fr_percent' : [] | [Decimal] }
  } |
  { 'Assigned' : { 'node_metrics' : [] | [NodeMetricsDaily] } };
export interface _SERVICE {
  'get_node_provider_rewards_calculation' : ActorMethod<
    [GetNodeProviderRewardsCalculationRequest],
    GetNodeProviderRewardsCalculationResponse
  >,
  'get_node_providers_monthly_xdr_rewards' : ActorMethod<
    [GetNodeProvidersMonthlyXdrRewardsRequest],
    GetNodeProvidersMonthlyXdrRewardsResponse
  >,
  'get_node_providers_rewards' : ActorMethod<
    [GetNodeProvidersRewardsRequest],
    GetNodeProvidersRewardsResponse
  >,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
