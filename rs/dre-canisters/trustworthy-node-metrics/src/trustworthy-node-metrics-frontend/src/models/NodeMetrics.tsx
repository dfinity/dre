import { computeAverageFailureRate } from '../utils/utils';
import { DailyNodeData } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import { Principal } from '@dfinity/principal';
export interface ChartData {
  date: Date ;
  failureRate: number | null;
}

export class DashboardNodeRewards {
  nodeId: Principal;
  nodeIdSmall: string;
  dailyData: DailyNodeData[];
  failureRateAvg: number;
  rewardsNoPenalty: number;
  rewardsWithPenalty: number;

  constructor(
    nodeId: Principal,
    dailyData: DailyNodeData[],
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
