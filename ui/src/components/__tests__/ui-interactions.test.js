import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import EmptyState from '../common/EmptyState.svelte';
import StatusBar from '../layout/StatusBar.svelte';
import InteractionHarness from './InteractionHarness.svelte';

afterEach(cleanup);

describe('shared UI interactions', () => {
  it('keeps the primary session action explicitly labelled', () => {
    render(StatusBar, {
      statusMessage: 'Torrent is ready',
      statusType: 'idle',
      startFaking: vi.fn(),
      stopFaking: vi.fn(),
    });

    expect(screen.getByRole('button', { name: 'Start faking' }).textContent).toContain('Start');
    expect(screen.getByText('Torrent is ready')).toBeTruthy();
  });

  it('exposes the actions allowed by a paused session', () => {
    render(StatusBar, {
      statusMessage: 'Paused',
      statusType: 'paused',
      isRunning: true,
      isPaused: true,
      startFaking: vi.fn(),
      stopFaking: vi.fn(),
      pauseFaking: vi.fn(),
      resumeFaking: vi.fn(),
      manualUpdate: vi.fn(),
    });

    expect(screen.getByRole('button', { name: 'Resume faking' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Update stats' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Stop faking' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Pause faking' })).toBeNull();
  });

  it('reveals advanced content and closes dialogs with Escape', async () => {
    render(InteractionHarness);

    expect(screen.queryByText('Hidden details')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: /Advanced settings/ }));
    expect(screen.getByText('Hidden details')).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'Open dialog' }));
    const dialog = screen.getByRole('dialog', { name: 'Test dialog' });
    expect(dialog).toBeTruthy();
    await fireEvent.keyDown(screen.getByRole('button', { name: 'Confirm action' }), {
      key: 'Escape',
    });
    expect(screen.queryByRole('dialog', { name: 'Test dialog' })).toBeNull();
  });

  it('renders a concise guided empty state', () => {
    render(EmptyState, {
      title: 'Choose a torrent',
      description: 'Start with a local torrent file.',
      steps: ['Choose', 'Configure', 'Start'],
    });

    expect(screen.getByRole('heading', { name: 'Choose a torrent' })).toBeTruthy();
    expect(screen.getAllByRole('listitem')).toHaveLength(3);
  });
});
