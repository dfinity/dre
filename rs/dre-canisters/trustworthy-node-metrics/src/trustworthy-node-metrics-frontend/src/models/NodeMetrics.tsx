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

  public transformDailyData(data: DailyData[]): { [key: string]: string | number | Date | null | undefined }[] {
    return data.map(item => ({
      date: item.date,
      subnetId: item.subnetId,
      numBlockFailures: item.numBlockFailures,
      numBlocksProposed: item.numBlocksProposed,
      failureRate: item.failureRate,
    }));
  }
}

export class DashboardNodeMetrics {
  nodeId: string;
  nodeIdSmall: string;
  dailyData: DailyData[];
  failureRateSum: number;
  failureRateAvg: number;

  constructor(
    nodeId: string,
    dailyData: DailyData[]
  ) {
    this.nodeId = nodeId;
    this.nodeIdSmall = nodeId.split('-')[0];
    this.dailyData = dailyData;
    this.failureRateSum = dailyData.reduce((sum, item) => sum + item.failureRate, 0);
    this.failureRateAvg = computeAverageFailureRate(dailyData.map(elem => elem.failureRate));
  }

  public getChartSeries() {
    return {
      data: this.dailyData.map(daily => daily.failureRate),
      label: this.nodeId,
    };
  }
}
