import React from 'react';
import { Toolbar } from '@mui/material';
import { LocalizationProvider, DatePicker } from '@mui/x-date-pickers';
import { AdapterDateFns } from '@mui/x-date-pickers/AdapterDateFns';
import ArrowRightIcon from '@mui/icons-material/ArrowRight';

// Define the types for props
export interface PeriodFilter {
  dateStart: Date;
  dateEnd: Date;
}

interface FilterBarProps {
  filters: PeriodFilter;
  setFilters: React.Dispatch<React.SetStateAction<PeriodFilter>>;
  subnets: Set<string> | null; // Use string instead of String
}

const FilterBar: React.FC<FilterBarProps> = ({ filters, setFilters }) => {
  const handleDateStartChange = (newValue: Date | null) => {
    if (newValue !== null) {
      setFilters((prev) => ({ ...prev, dateStart: newValue }));
    }
  };

  const handleDateEndChange = (newValue: Date | null) => {
    if (newValue !== null) {
      setFilters((prev) => ({ ...prev, dateEnd: newValue }));
    }
  };

  return (
      <Toolbar>
        <LocalizationProvider dateAdapter={AdapterDateFns}>
          <DatePicker
            label="From"
            value={filters.dateStart}
            onChange={handleDateStartChange}
          />
          <ArrowRightIcon />
          <DatePicker
            label="To"
            value={filters.dateEnd}
            onChange={handleDateEndChange}
          />
        </LocalizationProvider>
      </Toolbar>
  );
};


export default FilterBar;