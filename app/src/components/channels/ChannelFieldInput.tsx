import type { FieldRequirement } from '../../types/channels';

interface ChannelFieldInputProps {
  field: FieldRequirement;
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
}

const ChannelFieldInput = ({ field, value, onChange, disabled }: ChannelFieldInputProps) => {
  if (field.field_type === 'boolean') {
    return (
      <div>
        <label className="flex items-center gap-2 cursor-pointer">
          <button
            type="button"
            role="switch"
            aria-checked={value === 'true'}
            disabled={disabled}
            onClick={() => onChange(value === 'true' ? 'false' : 'true')}
            className={`relative inline-flex h-5 w-9 shrink-0 rounded-full border-2 border-transparent transition-colors focus:outline-none focus:ring-2 focus:ring-primary-500/40 ${
              value === 'true'
                ? 'bg-primary-500'
                : 'bg-stone-300 dark:bg-neutral-700'
            }`}>
            <span
              className={`pointer-events-none inline-block h-4 w-4 rounded-full bg-white shadow-sm transition-transform ${
                value === 'true' ? 'translate-x-4' : 'translate-x-0'
              }`}
            />
          </button>
          <span className="text-xs text-stone-500 dark:text-neutral-400">{field.label}</span>
        </label>
      </div>
    );
  }

  return (
    <div>
      <label className="block text-xs text-stone-500 dark:text-neutral-400 mb-1">
        {field.label}
        {field.required && <span className="text-coral-500 ml-0.5">*</span>}
      </label>
      <input
        type={field.field_type === 'secret' ? 'password' : 'text'}
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder={field.placeholder || field.label}
        disabled={disabled}
        className="w-full rounded-lg border border-stone-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 px-3 py-2 text-sm text-stone-900 dark:text-neutral-100 placeholder:text-stone-400 dark:placeholder:text-neutral-500 focus:outline-none focus:border-primary-500/60 disabled:opacity-50"
      />
    </div>
  );
};

export default ChannelFieldInput;
