import { DailyNodeMetrics } from "../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did";
import { PeriodFilter } from "../components/FilterBar";
export interface ChartData {
  date: Date ;
  dailyNodeMetrics: DailyNodeMetrics | null;
}

export const dateToNanoseconds = (date: Date): bigint => {
  const millisecondsSinceEpoch = date.getTime();
  const nanosecondsSinceEpoch = BigInt(millisecondsSinceEpoch) * BigInt(1_000_000);
  return nanosecondsSinceEpoch;
};

export const generateChartData = (periodFilter: PeriodFilter, dailyData: DailyNodeMetrics[]): ChartData[] => {
    const { dateStart, dateEnd } = periodFilter;
    const chartData: ChartData[] = [];

    const currentDate = new Date(dateStart);

    while (currentDate <= dateEnd) {
      const dailyDataEntry = dailyData.find(data => {
        const date = new Date(Number(data.ts) / 1000000);
        
        return date.getUTCFullYear() === currentDate.getUTCFullYear() &&
        date.getUTCMonth() === currentDate.getUTCMonth() &&
        date.getUTCDate() === currentDate.getUTCDate()
      });
      
      chartData.push({
          date: new Date(currentDate),
          dailyNodeMetrics: dailyDataEntry ? dailyDataEntry : null,
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

export const formatDateToUTC = (date: Date): string => {
    const day = String(date.getUTCDate()).padStart(2, '0');
    const month = String(date.getUTCMonth() + 1).padStart(2, '0'); // Months are zero-indexed
    const year = date.getUTCFullYear();
  
    return `${day}-${month}-${year}`;
  };
  
export const computeAverageFailureRate = (data: number[]): number => {
    if (data.length === 0) return 0;
    const sum = data.reduce((acc, val) => acc + val, 0);
    return sum / data.length;
  };

