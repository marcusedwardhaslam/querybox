import React, { useState } from 'react';

export interface TableProps {
  children: React.ReactNode;
  striped?: boolean;
  bordered?: boolean;
  compact?: boolean;
  fixed?: boolean;
  className?: string;
}

export const Table: React.FC<TableProps> = ({
  children,
  striped = false,
  bordered = false,
  compact = false,
  fixed = false,
  className = '',
}) => {
  const stripedClass = striped ? 'table-striped' : '';
  const borderedClass = bordered ? 'table-bordered' : '';
  const compactClass = compact ? 'table-compact' : '';
  const fixedClass = fixed ? 'table-fixed' : '';
  
  const classes = [
    'table',
    stripedClass,
    borderedClass,
    compactClass,
    fixedClass,
    className
  ].filter(Boolean).join(' ');

  return (
    <div className="table-container">
      <table className={classes}>
        {children}
      </table>
    </div>
  );
};

export interface TableHeadProps {
  children: React.ReactNode;
}

export const TableHead: React.FC<TableHeadProps> = ({ children }) => {
  return <thead>{children}</thead>;
};

export interface TableBodyProps {
  children: React.ReactNode;
}

export const TableBody: React.FC<TableBodyProps> = ({ children }) => {
  return <tbody>{children}</tbody>;
};

export interface TableRowProps {
  children: React.ReactNode;
  className?: string;
}

export const TableRow: React.FC<TableRowProps> = ({ children, className = '' }) => {
  return <tr className={className}>{children}</tr>;
};

export interface TableHeaderCellProps {
  children: React.ReactNode;
  sortable?: boolean;
  onSort?: () => void;
  sortDirection?: 'asc' | 'desc' | null;
  className?: string;
}

export const TableHeaderCell: React.FC<TableHeaderCellProps> = ({
  children,
  sortable = false,
  onSort,
  sortDirection = null,
  className = '',
}) => {
  const sortableClass = sortable ? 'sortable' : '';
  const sortedClass = sortDirection ? `sorted-${sortDirection}` : '';
  const classes = [sortableClass, sortedClass, className].filter(Boolean).join(' ');

  const handleClick = () => {
    if (sortable && onSort) {
      onSort();
    }
  };

  return (
    <th className={classes} onClick={handleClick}>
      {children}
    </th>
  );
};

export interface TableCellProps {
  children: React.ReactNode;
  className?: string;
}

export const TableCell: React.FC<TableCellProps> = ({ children, className = '' }) => {
  return <td className={className}>{children}</td>;
};

export interface TablePaginationProps {
  currentPage: number;
  totalPages: number;
  onPageChange: (page: number) => void;
  totalItems?: number;
  itemsPerPage?: number;
  className?: string;
}

export const TablePagination: React.FC<TablePaginationProps> = ({
  currentPage,
  totalPages,
  onPageChange,
  totalItems,
  itemsPerPage,
  className = '',
}) => {
  const startItem = (currentPage - 1) * (itemsPerPage || 0) + 1;
  const endItem = Math.min(currentPage * (itemsPerPage || 0), totalItems || 0);

  return (
    <div className={`table-pagination ${className}`}>
      {totalItems && itemsPerPage && (
        <div className="pagination-info">
          Showing {startItem} to {endItem} of {totalItems} entries
        </div>
      )}
      <div className="pagination-controls">
        <button
          className="pagination-btn"
          onClick={() => onPageChange(currentPage - 1)}
          disabled={currentPage === 1}
        >
          Previous
        </button>
        {Array.from({ length: totalPages }, (_, i) => i + 1).map((page) => (
          <button
            key={page}
            className={`pagination-btn ${page === currentPage ? 'active' : ''}`}
            onClick={() => onPageChange(page)}
          >
            {page}
          </button>
        ))}
        <button
          className="pagination-btn"
          onClick={() => onPageChange(currentPage + 1)}
          disabled={currentPage === totalPages}
        >
          Next
        </button>
      </div>
    </div>
  );
};
