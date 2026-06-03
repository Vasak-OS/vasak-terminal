<script setup lang="ts">
import { useNotification } from '@/utils/useNotification';

const { notifications } = useNotification();
</script>

<template>
  <Teleport to="body">
    <div class="fixed bottom-4 left-1/2 -translate-x-1/2 z-50 flex flex-col gap-2 pointer-events-none">
      <TransitionGroup
        enter-active-class="transition-all duration-300 ease-out"
        leave-active-class="transition-all duration-200 ease-in"
        enter-from-class="opacity-0 translate-y-2"
        leave-to-class="opacity-0 translate-y-2"
        move-class="transition-all duration-300"
      >
        <div
          v-for="n in notifications"
          :key="n.id"
          :class="[
            'pointer-events-auto px-4 py-2 rounded-corner shadow-lg text-sm border backdrop-blur-sm',
            n.type === 'success'
              ? 'bg-status-success/20 border-status-success text-tx-main'
              : n.type === 'error'
                ? 'bg-status-error/20 border-status-error text-tx-main'
                : 'bg-primary/20 border-primary text-tx-main',
          ]"
        >
          {{ n.message }}
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>
