import React, { useState, useRef, useEffect } from 'react';
import { ChevronDown } from 'lucide-react';
import { PAGE_SIZE_OPTIONS, type PageSize } from '../hooks/usePagination';

interface PageSizeDropdownProps {
  value: number;
  onChange: (size: PageSize) => void;
  position?: 'top' | 'bottom';
}

export const PageSizeDropdown: React.FC<PageSizeDropdownProps> = ({
  value,
  onChange,
  position = 'bottom',
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSelect = (size: PageSize) => {
    onChange(size);
    setIsOpen(false);
  };

  const positionClasses = position === 'top'
    ? 'bottom-full mb-1'
    : 'top-full mt-1';

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-2 px-3 py-1.5 text-sm rounded-lg bg-gray-100 dark:bg-white/10 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-white/20 transition-colors"
      >
        <span className="text-gray-500 dark:text-gray-400">Show</span>
        <span className="font-medium">{value}</span>
        <ChevronDown className={`w-4 h-4 transition-transform ${isOpen ? 'rotate-180' : ''}`} />
      </button>
      {isOpen && (
        <div className={`absolute ${positionClasses} right-0 bg-white dark:bg-neutral-800 border border-gray-200 dark:border-neutral-700 rounded-lg shadow-lg overflow-hidden z-10 min-w-[60px]`}>
          {PAGE_SIZE_OPTIONS.map((size) => (
            <button
              key={size}
              onClick={() => handleSelect(size)}
              className={`block w-full px-4 py-2 text-sm text-left hover:bg-gray-100 dark:hover:bg-white/10 ${
                value === size ? 'text-sui-500 font-medium' : 'text-gray-700 dark:text-gray-300'
              }`}
            >
              {size}
            </button>
          ))}
        </div>
      )}
    </div>
  );
};
