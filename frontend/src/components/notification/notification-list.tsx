import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Trash2, CheckCircle, X } from 'lucide-react';
import { axios } from '@/lib/axios';
import { endpoints } from '@/lib/endpoints';
import { useI18n } from '@/i18n/context';

interface Notification {
  id: string;
  title: string;
  message: string;
  created_at: string;
  read_at: string | null;
  type_color: string;
}

const NotificationList: React.FC = () => {
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [loading, setLoading] = useState(true);
  const { t } = useI18n();

  useEffect(() => {
    const fetchNotifications = async () => {
      try {
        const response = await axios.get(endpoints.notifications.list);
        setNotifications(response.data.notifications);
      } catch (error) {
        console.error(t('error_fetching_notifications'), error);
        // In case of error, we could set some default notifications
        setNotifications([]);
      } finally {
        setLoading(false);
      }
    };

    fetchNotifications();
  }, []);

  const handleMarkAsRead = async (id: string) => {
    try {
      await axios.put(endpoints.notifications.markRead(id));
      // Update the notification locally to reflect the change
      setNotifications(prev => prev.map(notif =>
        notif.id === id ? { ...notif, read_at: new Date().toISOString() } : notif
      ));
    } catch (error) {
      console.error(t('error_marking_notification_as_read'), error);
    }
  };

  const handleDeleteNotification = async (id: string) => {
    try {
      await axios.delete(endpoints.notifications.detail(id));
      // Remove the notification from the list
      setNotifications(prev => prev.filter(notif => notif.id !== id));
    } catch (error) {
      console.error(t('error_deleting_notification'), error);
    }
  };

  const handleMarkAllAsRead = async () => {
    try {
      await axios.put(endpoints.notifications.markAllRead);
      // Update all notifications locally to reflect the change
      setNotifications(prev => prev.map(notif => ({ ...notif, read_at: new Date().toISOString() })));
    } catch (error) {
      console.error(t('error_marking_all_notifications_as_read'), error);
    }
  };

  const handleDeleteAllNotifications = async () => {
    try {
      // First, delete all notifications from the backend
      await axios.delete(endpoints.notifications.list);
      // Then clear the local state
      setNotifications([]);
    } catch (error) {
      console.error(t('error_deleting_all_notifications'), error);
    }
  };

  const getTypeColor = (type: string) => {
    switch (type) {
      case 'success':
        return 'bg-green-100 text-green-800 border-green-200';
      case 'warning':
        return 'bg-yellow-100 text-yellow-800 border-yellow-200';
      case 'error':
        return 'bg-red-100 text-red-800 border-red-200';
      default:
        return 'bg-blue-100 text-blue-800 border-blue-200';
    }
  };

  if (loading) {
    return (
      <div className="flex justify-center items-center h-32">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-gray-300"></div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {notifications.length > 0 ? (
        <>
          {/* Action buttons for all notifications */}
          <div className="flex gap-2 mb-4">
            <Button
              variant="outline"
              size="sm"
              onClick={handleMarkAllAsRead}
              disabled={notifications.every(n => n.read_at !== null)}
            >
              <CheckCircle className="h-4 w-4 mr-2" />
              {t('mark_all_read')}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleDeleteAllNotifications}
              className="text-destructive hover:text-destructive"
            >
              <Trash2 className="h-4 w-4 mr-2" />
              {t('delete_all')}
            </Button>
          </div>

          {notifications.map((notification) => (
            <Card
              key={notification.id}
              className={`${!notification.read_at ? 'border-l-4 border-l-blue-500' : ''}`}
            >
              <CardHeader className="pb-2">
                <div className="flex items-start justify-between">
                  <CardTitle className="text-sm">{notification.title}</CardTitle>
                  <div className="flex gap-2">
                    <Badge variant="secondary" className={`text-xs ${getTypeColor(notification.type_color)}`}>
                      {notification.type_color.charAt(0).toUpperCase() + notification.type_color.slice(1)}
                    </Badge>
                    {!notification.read_at && (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleMarkAsRead(notification.id)}
                        className="h-6 w-6 p-0"
                      >
                        <CheckCircle className="h-4 w-4" />
                      </Button>
                    )}
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleDeleteNotification(notification.id)}
                      className="h-6 w-6 p-0 text-destructive hover:text-destructive"
                    >
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
                <p className="text-xs text-muted-foreground">
                  {new Date(notification.created_at).toLocaleDateString()}
                </p>
              </CardHeader>
              <CardContent>
                <p className="text-sm">{notification.message}</p>
              </CardContent>
            </Card>
          ))}
        </>
      ) : (
        <div className="text-center py-8 text-muted-foreground">
          {t('no_notifications_yet')}
        </div>
      )}
    </div>
  );
};

export default NotificationList;