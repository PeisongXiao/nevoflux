// Bridge between daemon `canvas.video.*` push messages and the
// render tab (nevoflux://render/{job_id}).

const renderTabs = new Map(); // job_id -> tabId

/**
 * Open a render tab for `job_id`, tracking it for later targeting.
 * Tab is opened hidden (background) to keep user workflow uninterrupted.
 */
async function openRenderTab(jobId, compositionId) {
  const tab = await browser.tabs.create({
    url: `nevoflux://render/${jobId}?job_id=${encodeURIComponent(jobId)}&composition_id=${encodeURIComponent(compositionId)}`,
    active: false,
  });
  renderTabs.set(jobId, tab.id);
  return tab.id;
}

async function closeRenderTab(jobId) {
  const tabId = renderTabs.get(jobId);
  if (tabId != null) {
    try { await browser.tabs.remove(tabId); } catch (e) { /* tab already gone */ }
    renderTabs.delete(jobId);
  }
}

/**
 * Daemon push `canvas_video_open` -> open tab.
 */
export async function handleOpen(push) {
  const { job_id, composition_id } = push;
  const tabId = await openRenderTab(job_id, composition_id);
  return { ok: true, tab_id: tabId };
}

/**
 * Daemon push `canvas_video_load` -> forward to render tab.
 */
export async function handleLoad(push) {
  const { job_id, html, width, height } = push;
  const tabId = renderTabs.get(job_id);
  if (tabId == null) throw new Error(`no render tab for job ${job_id}`);
  await browser.tabs.sendMessage(tabId, {
    type: 'render.load_composition',
    target_job_id: job_id,
    html, width, height,
  });
}

/**
 * Daemon push `canvas_video_seek` -> forward to render tab,
 * which replies via sendFrameChunks.
 */
export async function handleSeek(push) {
  const { job_id, t, frame_idx, width, height } = push;
  const tabId = renderTabs.get(job_id);
  if (tabId == null) throw new Error(`no render tab for job ${job_id}`);
  const resp = await browser.tabs.sendMessage(tabId, {
    type: 'render.seek_and_capture',
    target_job_id: job_id,
    t, frame_idx, width, height,
  });
  return resp;
}

/**
 * Daemon push `canvas_video_close` -> close tab.
 */
export async function handleClose(push) {
  const { job_id } = push;
  await closeRenderTab(job_id);
}

/**
 * Runtime message listener: PNG chunks and ready signals from the
 * render page flow back through here to the daemon.
 */
export function installRuntimeListener(sendToAgent) {
  browser.runtime.onMessage.addListener(async (msg, sender) => {
    if (!msg || typeof msg !== 'object') return;
    switch (msg.type) {
      case 'bg:canvas_video_ready':
        await sendToAgent({
          type: 'canvas_video_ready',
          payload: msg.payload,
        });
        break;
      case 'bg:canvas_video_frame_chunk':
        await sendToAgent({
          type: 'canvas_video_frame_chunk',
          payload: msg.payload,
        });
        break;
    }
  });
}
