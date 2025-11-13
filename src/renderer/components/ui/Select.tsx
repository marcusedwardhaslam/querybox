import React, { useState, useRef, useEffect } from 'react';

export interface SelectProps extends React.SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  options?: Array<{ value: string; label: string }>;
}

export const Select: React.FC<SelectProps> = ({
  label,
  options = [],
  className = '',
  children,
  ...props
}) => {
  return (
    <div className="input-group">
      {label && <label className="input-label">{label}</label>}
      <div className={`select ${className}`}>
        <select {...props}>
          {children || options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
};

export interface DropdownProps {
  trigger: React.ReactNode;
  children: React.ReactNode;
  className?: string;
}

export const Dropdown: React.FC<DropdownProps> = ({
  trigger,
  children,
  className = '',
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener('click', handleClickOutside);
    }

    return () => {
      document.removeEventListener('click', handleClickOutside);
    };
  }, [isOpen]);

  const handleTriggerClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setIsOpen(!isOpen);
  };

  const handleMenuClick = (e: React.MouseEvent) => {
    e.stopPropagation();
  };

  return (
    <div className={`dropdown ${className}`} ref={dropdownRef}>
      <div onClick={handleTriggerClick} className="dropdown-trigger">
        {trigger}
      </div>
      <div className={`dropdown-menu ${isOpen ? 'dropdown-open' : ''}`} onClick={handleMenuClick}>
        {children}
      </div>
    </div>
  );
};

export interface DropdownItemProps {
  children: React.ReactNode;
  onClick?: () => void;
  active?: boolean;
  className?: string;
}

export const DropdownItem: React.FC<DropdownItemProps> = ({
  children,
  onClick,
  active = false,
  className = '',
}) => {
  const classes = [
    'dropdown-item',
    active ? 'dropdown-item-active' : '',
    className
  ].filter(Boolean).join(' ');

  return (
    <div className={classes} onClick={onClick}>
      {children}
    </div>
  );
};

export const DropdownHeader: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return <div className="dropdown-header">{children}</div>;
};

export const DropdownDivider: React.FC = () => {
  return <div className="dropdown-divider"></div>;
};
