import debug from 'debug';
import { useCallback, useState } from 'react';
import { AUTH_MODE_LABELS } from '../../lib/channels/definitions';
import { useT } from '../../lib/i18n/I18nContext';
import { channelConnectionsApi } from '../../services/api/channelConnectionsApi';
import {
  disconnectChannelConnection,
  setChannelConnectionStatus,
  upsertChannelConnection,
} from '../../store/channelConnectionsSlice';
import { useAppDispatch, useAppSelector } from '../../store/hooks';
import type { ChannelConnectionStatus, ChannelDefinition } from '../../types/channels';
import { restartCoreProcess } from '../../utils/tauriCommands/core';
import ChannelFieldInput from './ChannelFieldInput';
import ChannelStatusBadge from './ChannelStatusBadge';

const log = debug('channels:lark');

interface LarkConfigProps {
  definition: ChannelDefinition;
}

const LarkConfig = ({ definition }: LarkConfigProps) => {
  const { t } = useT();
  const dispatch = useAppDispatch();
  const channelConnections = useAppSelector(state => state.channelConnections);

  const [busyKeys, setBusyKeys] = useState<Record<string, boolean>>({});
  const [fieldValues, setFieldValues] = useState<Record<string, Record<string, string>>>({});
  const [error, setError] = useState<string | null>(null);

  const runBusy = useCallback(async (key: string, task: () => Promise<void>) => {
    setBusyKeys(prev => ({ ...prev, [key]: true }));
    setError(null);
    try {
      await task();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
    } finally {
      setBusyKeys(prev => ({ ...prev, [key]: false }));
    }
  }, []);

  const updateField = useCallback((compositeKey: string, fieldKey: string, value: string) => {
    setFieldValues(prev => ({
      ...prev,
      [compositeKey]: { ...(prev[compositeKey] ?? {}), [fieldKey]: value },
    }));
  }, []);

  const handleConnect = useCallback(
    (spec: (typeof definition.auth_modes)[number]) => {
      const key = `lark:${spec.mode}`;
      void runBusy(key, async () => {
        dispatch(
          setChannelConnectionStatus({
            channel: 'lark',
            authMode: spec.mode,
            status: 'connecting',
          })
        );

        const credentials: Record<string, string> = {};
        for (const field of spec.fields) {
          const val = fieldValues[key]?.[field.key]?.trim() ?? '';
          if (field.required && !val) {
            dispatch(
              setChannelConnectionStatus({
                channel: 'lark',
                authMode: spec.mode,
                status: 'error',
                lastError: `${field.label} is required`,
              })
            );
            return;
          }
          if (val) credentials[field.key] = val;
        }

        const result = await channelConnectionsApi.connectChannel('lark', {
          authMode: spec.mode,
          credentials: Object.keys(credentials).length > 0 ? credentials : undefined,
        });

        if (result.restart_required) {
          try {
            await restartCoreProcess();
            dispatch(
              upsertChannelConnection({
                channel: 'lark',
                authMode: spec.mode,
                patch: { status: 'connected', lastError: undefined, capabilities: ['read', 'write'] },
              })
            );
          } catch (restartErr) {
            const msg = restartErr instanceof Error ? restartErr.message : String(restartErr);
            setError(msg);
          }
        } else {
          dispatch(
            upsertChannelConnection({
              channel: 'lark',
              authMode: spec.mode,
              patch: { status: 'connected', lastError: undefined, capabilities: ['read', 'write'] },
            })
          );
        }
      });
    },
    [dispatch, fieldValues, runBusy]
  );

  const handleDisconnect = useCallback(
    (authMode: string) => {
      const key = `lark:${authMode}`;
      void runBusy(key, async () => {
        await channelConnectionsApi.disconnectChannel('lark', authMode);
        dispatch(disconnectChannelConnection({ channel: 'lark', authMode }));
      });
    },
    [dispatch, runBusy]
  );

  return (
    <div className="space-y-3">
      {error && (
        <div className="rounded-lg border border-coral-200 dark:border-coral-500/30 bg-coral-50 dark:bg-coral-500/10 px-4 py-3 text-sm text-coral-700 dark:text-coral-300">
          {error}
        </div>
      )}

      {definition.auth_modes.map(spec => {
        const compositeKey = `lark:${spec.mode}`;
        const connection = channelConnections.connections.lark?.[spec.mode];
        const status: ChannelConnectionStatus = connection?.status ?? 'disconnected';

        return (
          <div
            key={spec.mode}
            className="rounded-lg border border-stone-200 dark:border-neutral-800 bg-stone-50 dark:bg-neutral-800/60 p-3">
            <div className="flex items-start justify-between gap-3">
              <div>
                <p className="text-sm font-medium text-stone-900 dark:text-neutral-100">
                  {AUTH_MODE_LABELS[spec.mode] ?? spec.mode}
                </p>
                <p className="text-xs text-stone-500 dark:text-neutral-400 mt-1">
                  {spec.description}
                </p>
                {connection?.lastError && (
                  <p className="text-xs text-coral-600 mt-1">{connection.lastError}</p>
                )}
              </div>
              <ChannelStatusBadge status={status} />
            </div>

            {spec.fields.length > 0 && (
              <div className="mt-3 space-y-2">
                {spec.fields.map(field => (
                  <ChannelFieldInput
                    key={field.key}
                    field={field}
                    value={fieldValues[compositeKey]?.[field.key] ?? (field.field_type === 'boolean' ? 'false' : '')}
                    onChange={val => updateField(compositeKey, field.key, val)}
                    disabled={busyKeys[compositeKey]}
                  />
                ))}
              </div>
            )}

            <div className="mt-3 flex gap-2">
              <button
                type="button"
                disabled={busyKeys[compositeKey]}
                onClick={() => handleConnect(spec)}
                className="rounded-lg bg-primary-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-primary-600 disabled:opacity-50">
                {status === 'connected' ? t('channels.telegram.reconnect') : t('channels.telegram.connect')}
              </button>
              <button
                type="button"
                disabled={busyKeys[compositeKey] || status === 'disconnected'}
                onClick={() => handleDisconnect(spec.mode)}
                className="rounded-lg border border-stone-200 dark:border-neutral-800 px-3 py-1.5 text-xs font-medium text-stone-600 dark:text-neutral-300 hover:border-stone-300 dark:hover:border-neutral-700 disabled:opacity-50">
                {t('accounts.disconnect')}
              </button>
            </div>
          </div>
        );
      })}
    </div>
  );
};

export default LarkConfig;
