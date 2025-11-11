import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

export type DailyNodeFailureRate = {
    'SubnetMember' : { 'node_metrics' : [] | [NodeMetricsDaily] }
  } |
  { 'NonSubnetMember' : { 'extrapolated_failure_rate' : [] | [number] } };
export interface DailyNodeProviderRewards {
  'total_adjusted_rewards_xdr_permyriad' : [] | [bigint],
  'total_base_rewards_xdr_permyriad' : [] | [bigint],
  'base_rewards' : Array<NodeTypeRegionBaseRewards>,
  'daily_nodes_rewards' : Array<DailyNodeRewards>,
  'base_rewards_type3' : Array<Type3BaseRewards>,
}
export interface DailyNodeRewards {
  'region' : [] | [string],
  'performance_multiplier' : [] | [number],
  'node_id' : [] | [Principal],
  'daily_node_failure_rate' : [] | [DailyNodeFailureRate],
  'base_rewards_xdr_permyriad' : [] | [number],
  'node_reward_type' : [] | [string],
  'rewards_reduction' : [] | [number],
  'adjusted_rewards_xdr_permyriad' : [] | [number],
  'dc_id' : [] | [string],
}
export interface DailyResults {
  'provider_results' : Array<[Principal, DailyNodeProviderRewards]>,
  'subnets_failure_rate' : Array<[Principal, number]>,
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
  'original_failure_rate' : [] | [number],
  'num_blocks_proposed' : [] | [bigint],
  'subnet_assigned_failure_rate' : [] | [number],
  'num_blocks_failed' : [] | [bigint],
  'relative_failure_rate' : [] | [number],
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
  'daily_xdr_permyriad' : [] | [number],
  'node_reward_type' : [] | [string],
  'monthly_xdr_permyriad' : [] | [number],
}
export interface Type3BaseRewards {
  'region' : [] | [string],
  'daily_xdr_permyriad' : [] | [number],
  'nodes_count' : [] | [bigint],
  'avg_coefficient' : [] | [number],
  'avg_rewards_xdr_permyriad' : [] | [number],
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
