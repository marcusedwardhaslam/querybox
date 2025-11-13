import React from 'react';

export interface AlertProps {
  variant: 'info' | 'success' | 'warning' | 'error';
  title?: string;
  description?: string;
  children?: React.ReactNode;
  className?: string;
}

export const Alert: React.FC<AlertProps> = ({
  variant,
  title,
  description,
  children,
  className = '',
}) => {
  const variantClass = `alert-${variant}`;
  const classes = ['alert', variantClass, className].filter(Boolean).join(' ');

  return (
    <div className={classes}>
      <div>
        {title && <div className="alert-title">{title}</div>}
        {description && <div className="alert-description">{description}</div>}
        {children}
      </div>
    </div>
  );
};
