import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'dashboard',
      component: () => import('@/views/Dashboard.vue'),
    },
    {
      path: '/tasks',
      name: 'tasks',
      component: () => import('@/views/Tasks.vue'),
    },
    {
      path: '/tasks/:id',
      name: 'task-detail',
      component: () => import('@/views/TaskDetail.vue'),
    },
    {
      path: '/crashes',
      name: 'crashes',
      component: () => import('@/views/Crashes.vue'),
    },
  ],
})

export default router
