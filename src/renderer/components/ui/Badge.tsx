import React from 'react';

export interface BadgeProps {
  variant?: 'primary' | 'secondary' | 'success' | 'warning' | 'error';
  children: React.ReactNode;
  className?: string;
}

export const Badge: React.FC<BadgeProps> = ({
  variant = 'primary',
  children,
  className = '',
}) => {
  const variantClass = `badge-${variant}`;
  const classes = ['badge', variantClass, className].filter(Boolean).join(' ');

  return (
    <span className={classes}>
      {children}
    </span>
  );
};
