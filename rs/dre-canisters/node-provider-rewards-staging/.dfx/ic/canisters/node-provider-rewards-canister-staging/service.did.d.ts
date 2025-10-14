import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

export type DailyNodeFailureRate = {
    'SubnetMember' : { 'node_metrics' : [] | [NodeMetricsDaily] }
  } |
  { 'NonSubnetMember' : { 'extrapolated_fr_percent' : [] | [number] } };
export interface DailyNodeProviderRewards {
  'rewards_total_xdr_permyriad' : [] | [bigint],
  'base_rewards' : Array<NodeTypeRegionBaseRewards>,
  'daily_nodes_rewards' : Array<DailyNodeRewards>,
  'base_rewards_type3' : Array<Type3BaseRewards>,
}
export interface DailyNodeRewards {
  'region' : [] | [string],
  'rewards_reduction_percent' : [] | [number],
  'node_id' : [] | [Principal],
  'daily_node_fr' : [] | [DailyNodeFailureRate],
  'base_rewards_xdr_permyriad' : [] | [bigint],
  'node_reward_type' : [] | [string],
  'adjusted_rewards_xdr_permyriad' : [] | [bigint],
  'performance_multiplier_percent' : [] | [number],
  'dc_id' : [] | [string],
}
export interface DailyResults {
  'subnets_fr' : Array<[Principal, number]>,
  'provider_results' : Array<[Principal, DailyNodeProviderRewards]>,
}
export interface DateUtc {
  'day' : [] | [number],
  'month' : [] | [number],
  'year' : [] | [number],
}
export interface GetNodeProvidersMonthlyXdrRewardsRequest {
  'registry_version' : [] | [bigint],
}
export interface GetNodeProvidersMonthlyXdrRewardsResponse {
  'error' : [] | [string],
  'rewards' : [] | [NodeProvidersMonthlyXdrRewards],
}
export interface GetNodeProvidersRewardsCalculationRequest { 'day' : DateUtc }
export type GetNodeProvidersRewardsCalculationResponse = {
    'Ok' : DailyResults
  } |
  { 'Err' : string };
export interface GetNodeProvidersRewardsRequest {
  'to_day' : DateUtc,
  'from_day' : DateUtc,
}
export type GetNodeProvidersRewardsResponse = { 'Ok' : NodeProvidersRewards } |
  { 'Err' : string };
export interface NodeMetricsDaily {
  'subnet_assigned' : [] | [Principal],
  'original_fr_percent' : [] | [number],
  'num_blocks_proposed' : [] | [bigint],
  'subnet_assigned_fr_percent' : [] | [number],
  'relative_fr_percent' : [] | [number],
  'num_blocks_failed' : [] | [bigint],
}
export interface NodeProvidersMonthlyXdrRewards {
  'registry_version' : [] | [bigint],
  'rewards' : Array<[Principal, bigint]>,
}
export interface NodeProvidersRewards {
  'rewards_xdr_permyriad' : Array<[Principal, bigint]>,
}
export interface NodeTypeRegionBaseRewards {
  'region' : [] | [string],
  'daily_xdr_permyriad' : [] | [bigint],
  'node_reward_type' : [] | [string],
  'monthly_xdr_permyriad' : [] | [bigint],
}
export interface Type3BaseRewards {
  'region' : [] | [string],
  'daily_xdr_permyriad' : [] | [bigint],
  'nodes_count' : [] | [bigint],
  'avg_coefficient_percent' : [] | [number],
  'avg_rewards_xdr_permyriad' : [] | [bigint],
}
export interface _SERVICE {
  'get_node_providers_monthly_xdr_rewards' : ActorMethod<
    [GetNodeProvidersMonthlyXdrRewardsRequest],
    GetNodeProvidersMonthlyXdrRewardsResponse
  >,
  'get_node_providers_rewards' : ActorMethod<
    [GetNodeProvidersRewardsRequest],
    GetNodeProvidersRewardsResponse
  >,
  'get_node_providers_rewards_calculation' : ActorMethod<
    [GetNodeProvidersRewardsCalculationRequest],
    GetNodeProvidersRewardsCalculationResponse
  >,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
