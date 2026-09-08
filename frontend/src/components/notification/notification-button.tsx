import React, { useState, useEffect, useCallback } from 'react';
import { Bell } from 'lucide-react';
import { Button } from '@/components/ui/button';
import NotificationSlideout from '@/components/notification/notification-slideout';
import { axios } from '@/lib/axios';
import { endpoints } from '@/lib/endpoints';

const NotificationButton = () => {
  const [isOpen, setIsOpen] = useState(false);
  const [unreadCount, setUnreadCount] = useState(0);

  const fetchUnreadCount = useCallback(async () => {
    try {
      const response = await axios.get(endpoints.notifications.list);
      const count = response.data.notifications.filter((n: { read_at: string | null }) => n.read_at === null).length;
      setUnreadCount(count);
    } catch {
      // silently ignore
    }
  }, []);

  useEffect(() => {
    fetchUnreadCount();
  }, [fetchUnreadCount]);

  const handleClose = () => {
    setIsOpen(false);
    fetchUnreadCount();
  };

  return (
    <>
      <Button
        variant="ghost"
        size="icon"
        onClick={() => setIsOpen(!isOpen)}
        className="relative"
      >
        <Bell className="h-5 w-5" />
        {unreadCount > 0 && (
          <span className="absolute top-2 right-2 h-2 w-2 rounded-full bg-red-500" />
        )}
      </Button>
      <NotificationSlideout isOpen={isOpen} onClose={handleClose} />
    </>
  );
};

export default NotificationButton;