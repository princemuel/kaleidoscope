# Render an ASCII character based on escape iteration count.
def printDensity(iterationCount)
  if iterationCount > 8 then
    putchard(32)  # ' '
  else if iterationCount > 4 then
    putchard(46)  # '.'
  else if iterationCount > 2 then
    putchard(43)  # '+'
  else
    putchard(42); # '*'


# Determine whether a point diverges.
# Iterates z = z² + c in the complex plane.
def mandelConverger(zReal zImag iterationCount cReal cImag)
  if iterationCount > 255 |
     (zReal * zReal + zImag * zImag > 4) then
    iterationCount
  else
    mandelConverger(
      zReal * zReal - zImag * zImag + cReal,
      2 * zReal * zImag + cImag,
      iterationCount + 1,
      cReal,
      cImag
    );


# Return the number of iterations required for the point to escape.
def mandelConverge(pointReal pointImag)
  mandelConverger(
    pointReal,
    pointImag,
    0,
    pointReal,
    pointImag
  );


# Compute and plot the Mandelbrot set for a specified region.
def mandelPlot(
  minReal maxReal realStep
  minImag maxImag imagStep
)
  for imag = minImag, imag < maxImag, imagStep in (
    (
      for real = minReal, real < maxReal, realStep in
        printDensity(mandelConverge(real, imag))
    )
    : putchard(10)
  )


# Convenience wrapper for plotting the Mandelbrot set.
#
# realStart, imagStart -> top-left corner
# realScale, imagScale -> magnification / step size
def mandel(realStart imagStart realScale imagScale)
  mandelPlot(
    realStart,
    realStart + realScale * 78,
    realScale,
    imagStart,
    imagStart + imagScale * 40,
    imagScale
  );
