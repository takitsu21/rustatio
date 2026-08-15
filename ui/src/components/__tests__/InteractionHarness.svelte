<script>
  import BaseModal from '../common/BaseModal.svelte';
  import DisclosurePanel from '../common/DisclosurePanel.svelte';

  let modalOpen = $state(false);
  let disclosureOpen = $state(false);
</script>

<button onclick={() => (modalOpen = true)}>Open dialog</button>

<DisclosurePanel
  title="Advanced settings"
  description="Optional configuration"
  bind:open={disclosureOpen}
>
  <p>Hidden details</p>
</DisclosurePanel>

<BaseModal bind:open={modalOpen} titleId="test-dialog-title">
  <div>
    <h2 id="test-dialog-title">Test dialog</h2>
    <button>Confirm action</button>
  </div>
</BaseModal>
