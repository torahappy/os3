# 使用するパッケージ
import numpy as np
from numpy.typing import NDArray
import scipy.signal
from statsmodels.tsa.stattools import levinson_durbin
from collections import defaultdict
import matplotlib.pyplot as plt
from scipy.fftpack import fft
import scipy.io.wavfile as wavfile
import sounddevice as sd
from matplotlib.animation import FuncAnimation

# プリエンファシス(高域強調)
def preEmphasis(wave, p=0.97):
    # 係数 (1.0, -p) のFIRフィルタを作成
    return scipy.signal.lfilter([1.0, -p], 1, wave)

def my_autocorr(x, order):
  N = len(x)
  r = np.zeros(order)
  for i in range(order):
    r[i] = np.sum(x[0:N-i] * x[i:N])
  
  return r

def my_levinson(signal, order=10):
  R = my_autocorr(signal, order+1)  # 自己相関関数
  A = np.ones(2)  # LPC係数 A[0] == 1.
  
  A[1] = -R[1] / R[0]
  e = R[0] + R[1]*A[1]  # 残差
  
  for k in range(2, order+1):
    lam = -np.sum(A * R[k:0:-1]) / e
    U = np.hstack((A, 0))
    V = lam * U[::-1]
    A = U+V
    e = (1 - lam**2) * e

  return A, e

fs = 48000

dl = sd.query_devices()
for dev in dl:
    print(dev)
    if 'default' == dev['name']:
        print("found: ", dev)
        my_idx = dev['index']
        break

sd.default.samplerate = fs
sd.default.device = my_idx

ms = 30
ms_2 = 30

duration = 1 / 1000 * ms #再生時間[秒]
window = int(fs / 1000 * ms)
window_2 = int(fs / 1000 * ms_2)

# タップ数、サンプルレート、チャンネル数

arcoefs, coeffs_2 = [None, None]

fig, ax = plt.subplots()
def update(frame):
    v = sd.rec(window_2, samplerate=fs, channels=1, dtype=np.int16).mean(axis=1)

    # print(v.shape)
    sd.wait()

    wav_list = []
    
    # LPC係数を求める(lpcの次数は要調整)
    lpcOrder = 32
    #センター部分を使う
    voice_data = v[:window]
    #正規化
    voice_data = voice_data/abs(voice_data).max()
    #プリエンファシス
    p = 0.97
    voice_data = preEmphasis(voice_data, p)
    #ハミング窓
    hammingWindow = np.hamming(len(voice_data))
    voice_data = voice_data * hammingWindow    

    sample = len(voice_data)

    sigma_v, arcoefs, pacf, sigma, phi = levinson_durbin(voice_data, lpcOrder)
    coeffs_1 = np.hstack((1, arcoefs))
    error_1 = sigma_v * sample
    coeffs_2, error_2 = my_levinson(voice_data, lpcOrder)

    if error_1 < 0.1:
        return

    ax.clear()
    print("Variance 1: " + str(error_1))
    print("Variance 2: " + str(error_2))
    print("Coeffs 1: " + str(coeffs_1))
    print("Coeffs 2: " + str(coeffs_2))
    # LPC係数の振幅スペクトルを求める
    fscale = np.fft.fftfreq(sample, d = 1.0 / fs)[:sample//2]
    # オリジナル信号の対数スペクトル
    spec = np.abs(fft(voice_data, sample))
    logspec = 20 * np.log10(spec)
    ax.plot(fscale, logspec[:sample//2])
    # LPC対数スペクトル
    w, h = scipy.signal.freqz(np.sqrt(error_1), coeffs_1, sample, "whole")
    lpcspec = np.abs(h)
    loglpcspec = 20 * np.log10(lpcspec)
    #出力をプロットしてみて出力
    ax.plot(fscale, loglpcspec[:sample//2], "r", linewidth=2)
    maxId = scipy.signal.argrelmax(loglpcspec[:sample//2],order=3)
    if len(maxId[0]) > 1:
        maxId = maxId[0]
    #とりあえず4つ分ぐらいのフォルマントの位置を出力
        ax.axvline(fscale[maxId[0]], ls = "--", color = "navy")
        ax.axvline(fscale[maxId[1]], ls = "--", color = "navy")
    plt.show()
    return

anim = FuncAnimation(fig, update, frames = None, cache_frame_data=False, interval=0)
plt.show()

