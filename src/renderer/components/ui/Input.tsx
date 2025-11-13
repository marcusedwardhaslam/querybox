import React from 'react';

export interface InputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size'> {
  label?: string;
  hint?: string;
  error?: string;
  required?: boolean;
  inputSize?: 'sm' | 'md' | 'lg';
}

export const Input: React.FC<InputProps> = ({
  label,
  hint,
  error,
  required = false,
  inputSize = 'md',
  className = '',
  ...props
}) => {
  const sizeClass = inputSize === 'sm' ? 'input-sm' : inputSize === 'lg' ? 'input-lg' : '';
  const errorClass = error ? 'input-error' : '';
  const inputClasses = ['input', sizeClass, errorClass, className].filter(Boolean).join(' ');

  return (
    <div className="input-group">
      {label && (
        <label className={`input-label ${required ? 'required' : ''}`}>
          {label}
        </label>
      )}
      <input className={inputClasses} {...props} />
      {hint && !error && <span className="input-hint">{hint}</span>}
      {error && <span className="input-error">{error}</span>}
    </div>
  );
};

export interface TextAreaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  hint?: string;
  error?: string;
  required?: boolean;
}

export const TextArea: React.FC<TextAreaProps> = ({
  label,
  hint,
  error,
  required = false,
  className = '',
  ...props
}) => {
  const errorClass = error ? 'input-error' : '';
  const textareaClasses = ['textarea', errorClass, className].filter(Boolean).join(' ');

  return (
    <div className="input-group">
      {label && (
        <label className={`input-label ${required ? 'required' : ''}`}>
          {label}
        </label>
      )}
      <textarea className={textareaClasses} {...props} />
      {hint && !error && <span className="input-hint">{hint}</span>}
      {error && <span className="input-error">{error}</span>}
    </div>
  );
};

export interface CheckboxProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'type'> {
  label?: string;
}

export const Checkbox: React.FC<CheckboxProps> = ({
  label,
  className = '',
  id,
  ...props
}) => {
  const inputId = id || `checkbox-${Math.random().toString(36).substr(2, 9)}`;

  return (
    <label className={`checkbox ${className}`}>
      <input type="checkbox" id={inputId} {...props} />
      {label && <span className="checkbox-label">{label}</span>}
    </label>
  );
};

export interface RadioProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'type'> {
  label?: string;
}

export const Radio: React.FC<RadioProps> = ({
  label,
  className = '',
  id,
  ...props
}) => {
  const inputId = id || `radio-${Math.random().toString(36).substr(2, 9)}`;

  return (
    <label className={`radio ${className}`}>
      <input type="radio" id={inputId} {...props} />
      {label && <span className="radio-label">{label}</span>}
    </label>
  );
};
