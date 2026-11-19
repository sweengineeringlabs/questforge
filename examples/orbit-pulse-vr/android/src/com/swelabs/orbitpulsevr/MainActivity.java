package com.swelabs.orbitpulsevr;

import android.app.Activity;
import android.content.Context;
import android.graphics.Color;
import android.hardware.Sensor;
import android.hardware.SensorEvent;
import android.hardware.SensorEventListener;
import android.hardware.SensorManager;
import android.os.Bundle;
import android.view.GestureDetector;
import android.view.MotionEvent;
import android.view.View;
import android.view.Gravity;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;

/**
 * Android launcher activity for Orbit Pulse VR with dual-mode hand wave detection:
 *  1. OpenXR XR_EXT_hand_tracking kinematics in native Rust (liborbit_pulse_vr.so).
 *  2. Hands-free physical hand wave detection via Sensor.TYPE_PROXIMITY (<5cm IR pass).
 *  3. Touch gesture fling/swipe fallback for non-sensor environments.
 */
public class MainActivity extends Activity implements SensorEventListener {

    private static boolean sNativeLoaded = false;
    static {
        try {
            System.loadLibrary("orbit_pulse_vr");
            sNativeLoaded = true;
        } catch (UnsatisfiedLinkError e) {
            sNativeLoaded = false;
        }
    }

    public static native void nativeInitGame();
    public static native int nativeDispatchAction(int actionCode);
    public static native int nativeOnGesture(int gestureId);

    private int mScore = 0;
    private int mCombo = 1;
    private int mShieldCharge = 100;
    private int mWaveGestureCount = 0;

    private SensorManager mSensorManager;
    private Sensor mProximitySensor;
    private GestureDetector mGestureDetector;
    private boolean mHandNear = false;

    private TextView mTvTitle;
    private TextView mTvStatus;
    private TextView mTvGesture;

    private static class FlingListener extends GestureDetector.SimpleOnGestureListener {
        private final MainActivity mActivity;

        FlingListener(MainActivity activity) {
            mActivity = activity;
        }

        @Override
        public boolean onFling(MotionEvent e1, MotionEvent e2, float velocityX, float velocityY) {
            if (Math.abs(velocityX) > 500) {
                mActivity.onHandWaveDetected("Touch Swipe Wave (" + (int) velocityX + " px/s)");
                return true;
            }
            return false;
        }
    }

    private static class ActionClickListener implements View.OnClickListener {
        private final MainActivity mActivity;
        private final int mAction;

        ActionClickListener(MainActivity activity, int action) {
            mActivity = activity;
            mAction = action;
        }

        @Override
        public void onClick(View v) {
            if (mAction == 4) {
                mActivity.onHandWaveDetected("Simulated Hand Wave");
            } else {
                mActivity.dispatchAction(mAction);
            }
        }
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        if (sNativeLoaded) {
            try {
                nativeInitGame();
            } catch (Throwable ignored) {}
        }

        setupSensors();
        buildUi();
    }

    private void setupSensors() {
        mSensorManager = (SensorManager) getSystemService(Context.SENSOR_SERVICE);
        if (mSensorManager != null) {
            mProximitySensor = mSensorManager.getDefaultSensor(Sensor.TYPE_PROXIMITY);
        }
        mGestureDetector = new GestureDetector(this, new FlingListener(this));
    }

    private void buildUi() {
        LinearLayout layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        layout.setPadding(40, 48, 40, 48);
        layout.setBackgroundColor(Color.parseColor("#0B1020"));
        layout.setGravity(Gravity.CENTER_HORIZONTAL);

        mTvTitle = new TextView(this);
        mTvTitle.setText("ORBIT PULSE VR\nQuestForge Mixed Reality Runtime");
        mTvTitle.setTextColor(Color.parseColor("#38BDF8"));
        mTvTitle.setTextSize(22f);
        mTvTitle.setGravity(Gravity.CENTER_HORIZONTAL);
        mTvTitle.setPadding(0, 0, 0, 24);
        layout.addView(mTvTitle);

        TextView tvMeta = new TextView(this);
        tvMeta.setText("120Hz  |  Passthrough  |  Hand Tracking EXT  |  Scene: OrbitArena");
        tvMeta.setTextColor(Color.parseColor("#94A3B8"));
        tvMeta.setTextSize(13f);
        tvMeta.setGravity(Gravity.CENTER_HORIZONTAL);
        tvMeta.setPadding(0, 0, 0, 28);
        layout.addView(tvMeta);

        mTvStatus = new TextView(this);
        mTvStatus.setText(formatScoreText());
        mTvStatus.setTextColor(Color.parseColor("#4ADE80"));
        mTvStatus.setTextSize(18f);
        mTvStatus.setGravity(Gravity.CENTER_HORIZONTAL);
        mTvStatus.setPadding(0, 0, 0, 20);
        layout.addView(mTvStatus);

        mTvGesture = new TextView(this);
        mTvGesture.setText("Sensor Hand Wave: [READY]\nWave hand over proximity sensor (<5cm) or swipe screen");
        mTvGesture.setTextColor(Color.parseColor("#FBBF24"));
        mTvGesture.setTextSize(14f);
        mTvGesture.setGravity(Gravity.CENTER_HORIZONTAL);
        mTvGesture.setPadding(0, 0, 0, 28);
        layout.addView(mTvGesture);

        Button btnSimulateWave = new Button(this);
        btnSimulateWave.setText("Wave Hand Gesture (Shield Pulse +20%)");
        btnSimulateWave.setBackgroundColor(Color.parseColor("#1E293B"));
        btnSimulateWave.setTextColor(Color.parseColor("#38BDF8"));
        btnSimulateWave.setPadding(20, 24, 20, 24);
        btnSimulateWave.setOnClickListener(new ActionClickListener(this, 4));
        layout.addView(btnSimulateWave);

        Button btnPulseOrb = new Button(this);
        btnPulseOrb.setText("Grab Pulse Orb (+100 Combo)");
        btnPulseOrb.setBackgroundColor(Color.parseColor("#1E293B"));
        btnPulseOrb.setTextColor(Color.parseColor("#E2E8F0"));
        btnPulseOrb.setOnClickListener(new ActionClickListener(this, 1));
        layout.addView(btnPulseOrb);

        Button btnShield = new Button(this);
        btnShield.setText("Activate Shield Emitter (+25)");
        btnShield.setBackgroundColor(Color.parseColor("#1E293B"));
        btnShield.setTextColor(Color.parseColor("#E2E8F0"));
        btnShield.setOnClickListener(new ActionClickListener(this, 2));
        layout.addView(btnShield);

        Button btnReset = new Button(this);
        btnReset.setText("Reset Arena Match");
        btnReset.setBackgroundColor(Color.parseColor("#1E293B"));
        btnReset.setTextColor(Color.parseColor("#F87171"));
        btnReset.setOnClickListener(new ActionClickListener(this, 5));
        layout.addView(btnReset);

        setContentView(layout);
    }

    @Override
    public boolean onTouchEvent(MotionEvent event) {
        if (mGestureDetector != null && mGestureDetector.onTouchEvent(event)) {
            return true;
        }
        return super.onTouchEvent(event);
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (mSensorManager != null && mProximitySensor != null) {
            mSensorManager.registerListener(this, mProximitySensor, SensorManager.SENSOR_DELAY_UI);
        }
    }

    @Override
    protected void onPause() {
        super.onPause();
        if (mSensorManager != null) {
            mSensorManager.unregisterListener(this);
        }
    }

    @Override
    public void onSensorChanged(SensorEvent event) {
        if (event.sensor.getType() == Sensor.TYPE_PROXIMITY) {
            float distance = event.values[0];
            float maxRange = event.sensor.getMaximumRange();
            boolean isNear = distance < Math.min(5.0f, maxRange);

            if (isNear && !mHandNear) {
                mHandNear = true;
            } else if (!isNear && mHandNear) {
                mHandNear = false;
                onHandWaveDetected("IR Proximity Sensor Wave (<5cm pass)");
            }
        }
    }

    @Override
    public void onAccuracyChanged(Sensor sensor, int accuracy) {}

    public void onHandWaveDetected(String source) {
        mWaveGestureCount++;
        mShieldCharge = Math.min(100, mShieldCharge + 20);
        mScore += 50 * mCombo;

        if (sNativeLoaded) {
            try {
                mScore = nativeOnGesture(1);
            } catch (Throwable ignored) {}
        }

        mTvStatus.setText(formatScoreText());
        mTvGesture.setText("WAVE DETECTED via " + source + "!\nWave Count: " + mWaveGestureCount + " | Kinetic Shield Boost +20%");
        Toast.makeText(this, "Hand Wave Recognized! (+50 pts)", Toast.LENGTH_SHORT).show();
    }

    public void dispatchAction(int actionCode) {
        if (actionCode == 1) {
            mScore += 100 * mCombo;
            mCombo = Math.min(8, mCombo + 1);
        } else if (actionCode == 2) {
            mShieldCharge = Math.max(0, mShieldCharge - 10);
            mScore += 25;
        } else if (actionCode == 5) {
            mScore = 0;
            mCombo = 1;
            mShieldCharge = 100;
            mWaveGestureCount = 0;
        }

        if (sNativeLoaded) {
            try {
                mScore = nativeDispatchAction(actionCode);
            } catch (Throwable ignored) {}
        }

        mTvStatus.setText(formatScoreText());
        if (actionCode == 5) {
            mTvGesture.setText("Sensor Hand Wave: [READY]\nWave hand over proximity sensor (<5cm) or swipe screen");
        }
    }

    private String formatScoreText() {
        return "Score: " + mScore + "   Combo: x" + mCombo + "   Shield: " + mShieldCharge + "%";
    }
}
