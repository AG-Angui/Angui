import { useEffect, useRef } from "react";
import { getSpaceEvents, getSpaceSocketTicket, type SpaceEvent } from "../api/collaborationSpaces";

/** Uses a one-time socket ticket and compensates missed versions after reconnect. */
export function useCollaborationSocket(token: string, spaceId: string, afterVersion: number, onEvents: (events: SpaceEvent[]) => void) {
  const callback = useRef(onEvents);
  const version = useRef(afterVersion);
  callback.current = onEvents;
  version.current = Math.max(version.current, afterVersion);
  useEffect(() => {
    let closed = false;
    let socket: WebSocket | null = null;
    let timer: number | null = null;
    let heartbeat: number | null = null;
    let backoff = 1_000;
    const retry = () => {
      if (closed || timer !== null) return;
      timer = window.setTimeout(() => {
        timer = null;
        void connect();
      }, backoff);
      backoff = Math.min(backoff * 2, 30_000);
    };
    const connect = async () => {
      try {
        const { ticket } = await getSpaceSocketTicket(token, spaceId);
        if (closed) return;
        const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        socket = new WebSocket(`${protocol}//${window.location.host}/api/collaboration-spaces/${encodeURIComponent(spaceId)}/events/ws?ticket=${encodeURIComponent(ticket)}&after_version=${version.current}`);
        socket.onopen = () => {
          backoff = 1_000;
          heartbeat = window.setInterval(() => socket?.readyState === WebSocket.OPEN && socket.send("ping"), 20_000);
          void getSpaceEvents(token, spaceId, version.current).then((events) => {
            if (events.length) {
              version.current = Math.max(version.current, ...events.map((event) => event.version));
              callback.current(events);
            }
          }).catch(() => undefined);
        };
        socket.onmessage = (event) => {
          try {
            const value: unknown = JSON.parse(event.data as string);
            if (Array.isArray(value)) {
              const events = value as SpaceEvent[];
              if (events.length) version.current = Math.max(version.current, ...events.map((item) => item.version));
              callback.current(events);
            }
          } catch { /* HTTP compensation handles malformed frames. */ }
        };
        socket.onclose = () => {
          if (heartbeat !== null) window.clearInterval(heartbeat);
          heartbeat = null;
          retry();
        };
        socket.onerror = () => socket?.close();
      } catch { retry(); }
    };
    void connect();
    return () => {
      closed = true;
      if (timer !== null) window.clearTimeout(timer);
      if (heartbeat !== null) window.clearInterval(heartbeat);
      socket?.close();
    };
  }, [token, spaceId]);
}
