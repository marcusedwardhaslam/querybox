import React from 'react';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'outline' | 'ghost' | 'danger' | 'success';
  size?: 'sm' | 'md' | 'lg';
  fullWidth?: boolean;
  icon?: boolean;
}

export const Button: React.FC<ButtonProps> = ({
  variant = 'primary',
  size = 'md',
  fullWidth = false,
  icon = false,
  className = '',
  children,
  ...props
}) => {
  const variantClass = `btn-${variant}`;
  const sizeClass = size === 'sm' ? 'btn-sm' : size === 'lg' ? 'btn-lg' : '';
  const fullWidthClass = fullWidth ? 'btn-block' : '';
  const iconClass = icon ? 'btn-icon' : '';
  
  const classes = [
    'btn',
    variantClass,
    sizeClass,
    fullWidthClass,
    iconClass,
    className
  ].filter(Boolean).join(' ');

  return (
    <button className={classes} {...props}>
      {children}
    </button>
  );
};

export interface ButtonGroupProps {
  children: React.ReactNode;
  className?: string;
}

export const ButtonGroup: React.FC<ButtonGroupProps> = ({ children, className = '' }) => {
  return (
    <div className={`btn-group ${className}`}>
      {children}
    </div>
  );
};
