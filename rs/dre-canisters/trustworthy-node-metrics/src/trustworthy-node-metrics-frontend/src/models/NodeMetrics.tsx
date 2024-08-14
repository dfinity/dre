import { DailyNodeMetrics } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import { computeAverageFailureRate } from '../utils/utils';
import { Principal } from '@dfinity/principal';
export interface ChartData {
  date: Date ;
  failureRate: number | null;
}

export class DashboardNodeRewards {
  nodeId: Principal;
  nodeIdSmall: string;
  dailyData: DailyNodeMetrics[];
  failureRateAvg: number;
  rewardsNoPenalty: number;
  rewardsWithPenalty: number;

  constructor(
    nodeId: Principal,
    dailyData: DailyNodeMetrics[],
    rewardsNoPenalty: number,
    rewardsWithPenalty: number,
  ) {
    this.nodeId = nodeId;
    this.nodeIdSmall = nodeId.toText().split('-')[0];
    this.dailyData = dailyData;
    this.failureRateAvg = computeAverageFailureRate(dailyData.map(elem => elem.failure_rate));
    this.rewardsNoPenalty = rewardsNoPenalty;
    this.rewardsWithPenalty = rewardsWithPenalty;
  }
}
