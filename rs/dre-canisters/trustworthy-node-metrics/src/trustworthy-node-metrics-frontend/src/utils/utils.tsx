import { PeriodFilter } from "../components/FilterBar";
import { ChartData, DailyData, NodeMetrics } from "../models/NodeMetrics";

export const generateChartData = (periodFilter: PeriodFilter, dailyData: DailyData[]): ChartData[] => {
    const { dateStart, dateEnd } = periodFilter;
    const chartData: ChartData[] = [];

    const currentDate = new Date(dateStart);

    while (currentDate <= dateEnd) {
        const dailyDataEntry = dailyData.find(data => 
            data.date.getFullYear() === currentDate.getFullYear() &&
            data.date.getMonth() === currentDate.getMonth() &&
            data.date.getDate() === currentDate.getDate()
        );

        chartData.push({
            date: new Date(currentDate),
            failureRate: dailyDataEntry ? dailyDataEntry.failureRate : null,
        });

        currentDate.setDate(currentDate.getDate() + 1);
    }

    return chartData;
};


export const getFormattedDates = (period: PeriodFilter): string[] => {
    const { dateStart, dateEnd } = period;
    const dates: string[] = [];
    const currentDate = new Date(dateStart);
  
    while (currentDate <= dateEnd) {
      dates.push(
        currentDate.toLocaleDateString('UTC', { month: 'short', day: 'numeric' }).replace(" ", "\n")
      );
      currentDate.setDate(currentDate.getDate() + 1);
    }
  
    return dates;
  };

export function transformDailyData(data: DailyData[]): { [key: string]: string | number | Date | null | undefined }[] {
    return data.map(item => ({
      date: item.date,
      subnetId: item.subnetId,
      numBlockFailures: item.numBlockFailures,
      numBlocksProposed: item.numBlocksProposed,
      failureRate: item.failureRate,
    }));
  }

export function groupBy<T, K extends keyof T>(items: T[], key: K): Record<string, T[]> {
    return items.reduce((result, item) => {
      const groupKey = String(item[key]);
      if (!result[groupKey]) {
        result[groupKey] = [];
      }
      result[groupKey].push(item);
      return result;
    }, {} as Record<string, T[]>);
  }
  

  export function calculateDailyValues (items: NodeMetrics[],): DailyData[] {
    const dailyValues: DailyData[] = [];
    let previousTotals = { failures: 0.0, proposed: 0.0 };
    
    items.sort((a, b) => a.date.getTime() - b.date.getTime());
  
    for (const item of items) {
      const currentDate = item.date;
      const currentSubnet = item.subnetId;
      const currentTotals = {
        failures: Number(item.numBlockFailuresTotal),
        proposed: Number(item.numBlocksProposedTotal),
      };
  
      if (previousTotals.failures || previousTotals.proposed) {
          const dailyFailures = currentTotals.failures - previousTotals.failures;
          const dailyProposed = currentTotals.proposed - previousTotals.proposed;
  
          dailyValues.push( new DailyData(
              currentDate,
              currentSubnet.toText(),
              dailyFailures,
              dailyProposed,
          ));
      }
  
      previousTotals = currentTotals;
    }
  
    return dailyValues;
  }
  
  export const computeAverageFailureRate = (data: number[]): number => {
      if (data.length === 0) return 0;
      const sum = data.reduce((acc, val) => acc + val, 0);
      return sum / data.length;
    };
  
