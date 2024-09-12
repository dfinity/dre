import React, { useState } from 'react';
import { Box, Snackbar, Alert } from '@mui/material';
import { LocalizationProvider, DatePicker, ArrowRightIcon } from '@mui/x-date-pickers';
import { AdapterDateFns } from '@mui/x-date-pickers/AdapterDateFns';
import { addMonths, isAfter, differenceInMonths } from 'date-fns';

export interface PeriodFilter {
  dateStart: Date;
  dateEnd: Date;
}

interface FilterBarProps {
  filters: PeriodFilter;
  setFilters: React.Dispatch<React.SetStateAction<PeriodFilter>>;
}

const makeLocalAppearUTC = (date: Date) => {
  return new Date(date.getTime() + date.getTimezoneOffset() * 60000);
};

const localToUTC = (date: Date) => {
  return new Date(date.getTime() - date.getTimezoneOffset() * 60000);
};

const FilterBar: React.FC<FilterBarProps> = ({ filters, setFilters }) => {
  const [error, setError] = useState<string | null>(null);

  const handleDateStartChange = (newValue: Date | null) => {
    if (newValue) {
      const startValue = localToUTC(newValue);
      const updatedEndDate = filters.dateEnd;
      if (updatedEndDate && isAfter(updatedEndDate, startValue) && differenceInMonths(updatedEndDate, startValue) <= 2) {
        setFilters((prev) => ({ ...prev, dateStart: startValue }));
        setError(null);
      } else {
        setError('The "To" date must be within 2 months of the "From" date.');
        setFilters((prev) => ({ ...prev, dateStart: startValue, dateEnd: addMonths(startValue, 2) }));
      }
    }
  };

  const handleDateEndChange = (newValue: Date | null) => {
    if (newValue) {
      newValue.setUTCMilliseconds(999);
      const endValue = localToUTC(newValue);

      if (filters.dateStart) {
        if (isAfter(endValue, filters.dateStart)) {
          if (differenceInMonths(endValue, filters.dateStart) <= 2) {
            setFilters((prev) => ({ ...prev, dateEnd: endValue }));
            setError(null);
          } else {
            setError('The "To" date must be within 2 months of the "From" date.');
            setFilters((prev) => ({ ...prev, dateEnd: addMonths(filters.dateStart, 2) }));
          }
        } else {
          setError('The "To" date must be after the "From" date.');
        }
      }
    }
  };

  return (
    <Box mt={2}>
      <LocalizationProvider dateAdapter={AdapterDateFns}>
        <Box display="flex" alignItems="center">
          <DatePicker
            label="From"
            value={makeLocalAppearUTC(filters.dateStart)}
            onChange={handleDateStartChange}
            format="dd/MM/yyy"
          />
          <ArrowRightIcon />
          <DatePicker
            label="To"
            value={makeLocalAppearUTC(filters.dateEnd)}
            onChange={handleDateEndChange}
            format="dd/MM/yyy"
          />
        </Box>
      </LocalizationProvider>

      <Snackbar open={!!error} autoHideDuration={6000} onClose={() => setError(null)} anchorOrigin={{ vertical: 'top', horizontal: 'center' }}>
        <Alert onClose={() => setError(null)} severity="error" sx={{ width: '100%' }}>
          {error}
        </Alert>
      </Snackbar>
    </Box>
  );
};

export default FilterBar;
