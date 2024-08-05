import { Principal } from '@dfinity/principal';
import { computeAverageFailureRate } from '../utils/utils';

export class NodeMetrics {
  date: Date;
  numBlockFailuresTotal: bigint;
  nodeId: Principal;
  numBlocksProposedTotal: bigint;
  subnetId: Principal

  constructor(
    ts: bigint,
    numBlockFailuresTotal: bigint,
    nodeId: Principal,
    numBlocksProposedTotal: bigint,
    subnetId: Principal
  ) {
    this.date = new Date(Number(ts / BigInt(1e6)));
    this.numBlockFailuresTotal = numBlockFailuresTotal;
    this.nodeId = nodeId;
    this.numBlocksProposedTotal = numBlocksProposedTotal;
    this.subnetId = subnetId;
  }
}

export interface ChartData {
  date: Date ;
  failureRate: number | null;
}

export class DailyData {
  date: Date;
  subnetId: string;
  numBlockFailures: number;
  numBlocksProposed: number;
  failureRate: number;

  constructor(
    date: Date,
    subnetId: string,
    numBlockFailures: number,
    numBlocksProposed: number
  ) {
    this.date = date;
    this.subnetId = subnetId;
    this.numBlockFailures = numBlockFailures;
    this.numBlocksProposed = numBlocksProposed;
    this.failureRate = numBlockFailures / (numBlockFailures + numBlocksProposed) * 100;
  }
}

export class DashboardNodeMetrics {
  nodeId: string;
  nodeIdSmall: string;
  dailyData: DailyData[];
  failureRateAvg: number;
  rewardsNoPenalty: number;

  constructor(
    nodeId: string,
    dailyData: DailyData[],
    rewardsNoPenalty: number,
  ) {
    this.nodeId = nodeId;
    this.nodeIdSmall = nodeId.split('-')[0];
    this.dailyData = dailyData;
    this.failureRateAvg = computeAverageFailureRate(dailyData.map(elem => elem.failureRate));
    this.rewardsNoPenalty = rewardsNoPenalty;
  }

  public getChartSeries() {
    return {
      data: this.dailyData.map(daily => daily.failureRate),
      label: this.nodeId,
    };
  }
}
