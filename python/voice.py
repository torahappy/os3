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

# プリエンファシス(高域強調)
def preEmphasis(wave, p=0.97):
    # 係数 (1.0, -p) のFIRフィルタを作成
    return scipy.signal.lfilter([1.0, -p], 1, wave)

boin_list = ["a","i","u","e","o"]

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

  A = -A
  A[0] = 1.0

  return A, e

fs =44100
sd.default.samplerate = fs
sd.default.channels = 1
sd.default.device = 0
sd.default.dtype = 'float32'

dl = sd.query_devices()
for dev in dl:
    pass

ms = 40

duration = 1 / 1000 * ms #再生時間[秒]
window = int(fs / 1000 * ms)

# タップ数、サンプルレート、チャンネル数

# データの読み込み, 音素ごとに辞書化
for i in range(1,100,1):
    v = sd.rec(window)

    sd.wait()

    wav_list = []

    
    # LPC係数を求める(lpcの次数は要調整)
    lpcOrder = 32
    fig = plt.figure()
    ax = fig.add_subplot(1, 1, 1)
    #センター部分を使う
    cuttime = 0.04
    voice_data = v
    #正規化
    voice_data = voice_data/abs(voice_data).max()
    #プリエンファシス
    p = 0.97
    voice_data = preEmphasis(voice_data, p)
    #ハミング窓
    hammingWindow = np.hamming(len(voice_data))
    voice_data = voice_data * hammingWindow    
    sigma_v, arcoefs, pacf, sigma, phi = levinson_durbin(voice_data, lpcOrder)
    coeffs_2, error_2 = my_levinson(voice_data, lpcOrder)
    print("Variance 1: " + str(sigma_v))
    print("Variance 2: " + str(error_2))
    print("Coeffs 1: " + str(arcoefs))
    print("Coeffs 2: " + str(coeffs_2))




    """
    # LPC係数の振幅スペクトルを求める
    sample = len(voice_data)
    fscale = np.fft.fftfreq(sample, d = 1.0 / fs)[:sample//2]
    # オリジナル信号の対数スペクトル
    spec = np.abs(fft(voice_data, sample))
    logspec = 20 * np.log10(spec)
    ax.plot(fscale, logspec[:sample//2])
    # LPC対数スペクトル
    w, h = scipy.signal.freqz(np.sqrt(e), a, sample, "whole")
    lpcspec = np.abs(h)
    loglpcspec = 20 * np.log10(lpcspec)
    #出力をプロットしてみて出力
    ax.plot(fscale, loglpcspec[:sample//2], "r", linewidth=2)
    maxId = scipy.signal.argrelmax(loglpcspec[:sample//2],order=3)
    maxId = maxId[0]
    #とりあえず4つ分ぐらいのフォルマントの位置を出力
    ax.axvline(fscale[maxId[0]], ls = "--", color = "navy")
    ax.axvline(fscale[maxId[1]], ls = "--", color = "navy")
    ax.axvline(fscale[maxId[2]], ls = "--", color = "navy")
    ax.axvline(fscale[maxId[3]], ls = "--", color = "navy")
    plt.show()
    """
