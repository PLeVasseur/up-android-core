package org.eclipse.uprotocol.core.ustreamer;

import static android.content.Context.BIND_AUTO_CREATE;
import static androidx.core.content.ContentProviderCompat.requireContext;
import static org.eclipse.uprotocol.common.util.UStatusUtils.toStatus;
import static org.eclipse.uprotocol.common.util.log.Formatter.join;
import static org.eclipse.uprotocol.common.util.log.Formatter.tag;

import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.ServiceConnection;
import android.os.IBinder;
import android.util.Log;

import androidx.annotation.NonNull;

import org.eclipse.uprotocol.common.util.log.Key;
import org.eclipse.uprotocol.core.UCore;
import org.eclipse.uprotocol.v1.UEntity;

import java.util.logging.Logger;

public class UStreamer extends UCore.Component {
    public static final UEntity SERVICE = UEntity.newBuilder().setName("ustreamer").setVersionMajor(1).build();

    protected static final String TAG = tag(SERVICE.getName());
    protected static boolean VERBOSE = Log.isLoggable(TAG, Log.VERBOSE);

    private static final String USTREAMER_SERVICE_PACKAGE = "org.eclipse.uprotocol.streamer.service";

    private static final ComponentName USTREAMER_SERVICE_COMPONENT =
            new ComponentName(USTREAMER_SERVICE_PACKAGE, USTREAMER_SERVICE_PACKAGE + ".UStreamerService");

    final Context mContext;

    private final ServiceConnection mServiceConnectionListener = new ServiceConnection() {
        @Override
        public void onServiceConnected(ComponentName name, IBinder service) {
            Log.i(TAG, join(Key.EVENT, "Service started", Key.PACKAGE, name.getPackageName()));
        }

        @Override
        public void onServiceDisconnected(ComponentName name) {
            Log.i(TAG, join(Key.EVENT, "Service stopped", Key.PACKAGE, name.getPackageName()));
        }
    };

    private void startService() {
        try {
            final Intent intent = new Intent().setComponent(USTREAMER_SERVICE_COMPONENT);
            mContext.bindService(intent, mServiceConnectionListener, BIND_AUTO_CREATE);
        } catch (Exception e) {
            Log.e("bindService", USTREAMER_SERVICE_PACKAGE + toStatus(e));
        }
    }

    private void stopService() {
        mContext.unbindService(mServiceConnectionListener);
    }

    public UStreamer(@NonNull Context context) {
        mContext = context;
    }

    @Override
    protected void init(@NonNull UCore uCore) {
        Log.i(TAG, "Service init");
    }

    @Override
    protected void startup() {
        Log.i(TAG, join(Key.EVENT, "Service start"));
        startService();
    }

    @Override
    protected void shutdown() {
        Log.i(TAG, join(Key.EVENT, "Service shutdown"));
        stopService();
    }
}
