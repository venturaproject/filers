import { AxiosInstance } from 'axios';
import pathFor from '@/lib/app-routes';

declare global {
    interface Window {
        axios: AxiosInstance;
    }

    /* eslint-disable no-var */
    var route: typeof pathFor;
}
